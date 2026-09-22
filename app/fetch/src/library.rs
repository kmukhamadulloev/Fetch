use std::{path::Path, sync::Arc};

use fetch_core::{
    CompletedFile, CompletedOperations, FetchError, PlaybackProgress, PlaybackProgressUpdate,
};
use fetch_storage::Storage;
use tracing::info;
use uuid::Uuid;

#[async_trait::async_trait]
trait PathRevealer: Send + Sync {
    async fn reveal(&self, path: &Path) -> Result<(), FetchError>;
}

struct SystemPathRevealer;

#[async_trait::async_trait]
impl PathRevealer for SystemPathRevealer {
    async fn reveal(&self, path: &Path) -> Result<(), FetchError> {
        #[cfg(target_os = "macos")]
        let mut command = {
            let mut command = tokio::process::Command::new("open");
            command.arg("-R").arg(path);
            command
        };

        #[cfg(target_os = "windows")]
        let mut command = {
            let mut command = tokio::process::Command::new("explorer.exe");
            command.arg(format!("/select,{}", path.display()));
            command
        };

        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        let mut command = {
            let directory = path.parent().ok_or(FetchError::FileNotFound)?;
            let mut command = tokio::process::Command::new("xdg-open");
            command.arg(directory);
            command
        };

        let status = command
            .status()
            .await
            .map_err(|error| FetchError::ProcessFailed {
                summary: "Could not open the containing folder".into(),
                details: error.to_string(),
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(FetchError::ProcessFailed {
                summary: "Could not open the containing folder".into(),
                details: format!("file manager exited with {status}"),
            })
        }
    }
}

pub struct CompletedLibrary {
    storage: Arc<Storage>,
    revealer: Arc<dyn PathRevealer>,
    metadata: Option<crate::metadata::MetadataService>,
}

impl CompletedLibrary {
    #[cfg(test)]
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            revealer: Arc::new(SystemPathRevealer),
            metadata: None,
        }
    }

    #[cfg(test)]
    fn with_revealer(storage: Arc<Storage>, revealer: Arc<dyn PathRevealer>) -> Self {
        Self {
            storage,
            revealer,
            metadata: None,
        }
    }

    pub fn with_metadata(
        storage: Arc<Storage>,
        metadata: crate::metadata::MetadataService,
    ) -> Self {
        Self {
            storage,
            revealer: Arc::new(SystemPathRevealer),
            metadata: Some(metadata),
        }
    }

    fn metadata_service(&self) -> Result<&crate::metadata::MetadataService, FetchError> {
        self.metadata
            .as_ref()
            .ok_or_else(|| FetchError::InvalidRequest("Metadata editing is unavailable".into()))
    }

    async fn find(&self, id: Uuid) -> Result<CompletedFile, FetchError> {
        self.storage
            .get_completed_file(id)
            .await
            .map_err(storage_error)?
            .ok_or(FetchError::FileNotFound)
    }
}

#[async_trait::async_trait]
impl CompletedOperations for CompletedLibrary {
    async fn metadata(&self, id: Uuid) -> Result<fetch_core::MediaMetadata, FetchError> {
        self.metadata_service()?.inspect(id).await
    }
    async fn metadata_artwork(&self, id: Uuid) -> Result<Vec<u8>, FetchError> {
        self.metadata_service()?.artwork(id).await
    }
    async fn update_metadata(
        &self,
        id: Uuid,
        update: fetch_core::MetadataUpdate,
    ) -> Result<fetch_core::MetadataSaveStatus, FetchError> {
        self.metadata_service()?.start(id, update).await
    }
    async fn metadata_status(
        &self,
        id: Uuid,
    ) -> Result<fetch_core::MetadataSaveStatus, FetchError> {
        self.metadata_service()?.status(id).await
    }

    async fn list_completed(&self) -> Result<Vec<CompletedFile>, FetchError> {
        self.storage
            .list_completed_files()
            .await
            .map_err(storage_error)
    }

    async fn get_completed(&self, id: Uuid) -> Result<CompletedFile, FetchError> {
        self.find(id).await
    }

    async fn reveal_completed(&self, id: Uuid) -> Result<(), FetchError> {
        let file = self.find(id).await?;
        tokio::fs::metadata(&file.path)
            .await
            .map_err(|_| FetchError::FileNotFound)?;
        self.revealer.reveal(&file.path).await?;
        info!(file_id = %id, "opened completed file location");
        Ok(())
    }

    async fn delete_completed(&self, id: Uuid) -> Result<(), FetchError> {
        let _guard = self
            .metadata
            .as_ref()
            .map(|service| service.lock.try_lock().map_err(|_| crate::metadata::busy()))
            .transpose()?;
        let file = self.find(id).await?;
        remove_file_if_present(&file.path).await?;
        if let Some(thumbnail) = file.thumbnail_path.as_deref()
            && thumbnail != file.path
        {
            remove_file_if_present(thumbnail).await?;
        }
        if !self
            .storage
            .delete_completed_file(id)
            .await
            .map_err(storage_error)?
        {
            return Err(FetchError::FileNotFound);
        }
        info!(file_id = %id, filename = %file.filename, "deleted completed media");
        Ok(())
    }

    async fn save_playback_progress(
        &self,
        id: Uuid,
        update: PlaybackProgressUpdate,
    ) -> Result<PlaybackProgress, FetchError> {
        self.find(id).await?;
        if !update.position_seconds.is_finite()
            || !update.duration_seconds.is_finite()
            || update.position_seconds < 0.0
            || update.duration_seconds <= 0.0
            || update.position_seconds > update.duration_seconds + 1.0
        {
            return Err(FetchError::InvalidRequest(
                "playback position and duration are invalid".into(),
            ));
        }
        let completed = update.position_seconds >= update.duration_seconds * 0.95;
        let progress = PlaybackProgress {
            file_id: id,
            position_seconds: if completed {
                update.duration_seconds
            } else {
                update.position_seconds.min(update.duration_seconds)
            },
            duration_seconds: update.duration_seconds,
            completed,
            updated_at: chrono::Utc::now(),
        };
        self.storage
            .save_playback_progress(&progress)
            .await
            .map_err(storage_error)?;
        Ok(progress)
    }

    async fn clear_playback_progress(&self, id: Uuid) -> Result<(), FetchError> {
        self.find(id).await?;
        self.storage
            .delete_playback_progress(id)
            .await
            .map_err(storage_error)
    }
}

async fn remove_file_if_present(path: &Path) -> Result<(), FetchError> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FetchError::Internal(format!(
            "could not delete {}: {error}",
            path.display()
        ))),
    }
}

fn storage_error(error: fetch_storage::StorageError) -> FetchError {
    FetchError::Internal(error.to_string())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::Utc;
    use fetch_core::{DownloadJob, DownloadMode, DownloadRequest};
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct RecordingRevealer(Mutex<Vec<std::path::PathBuf>>);

    #[async_trait::async_trait]
    impl PathRevealer for RecordingRevealer {
        async fn reveal(&self, path: &Path) -> Result<(), FetchError> {
            self.0.lock().await.push(path.to_owned());
            Ok(())
        }
    }

    pub(crate) async fn fixture() -> (tempfile::TempDir, Arc<Storage>, CompletedFile) {
        let directory = tempfile::tempdir().unwrap();
        let storage = Arc::new(
            Storage::open(&directory.path().join("fetch.sqlite3"))
                .await
                .unwrap(),
        );
        let job = DownloadJob::new(DownloadRequest {
            url: "https://example.test/media".into(),
            title: Some("Fixture".into()),
            duration_seconds: Some(42.0),
            mode: DownloadMode::Video,
            format_id: None,
            quality: None,
            container: None,
            video_codec: None,
            audio_codec: None,
            embed_metadata: false,
            embed_thumbnail: false,
            subtitles: false,
            playlist: None,
            output_directory: Some(directory.path().into()),
        });
        storage.insert_job(&job).await.unwrap();
        let media = directory.path().join("media.mp4");
        let thumbnail = directory.path().join("thumbnail.jpg");
        tokio::fs::write(&media, b"media").await.unwrap();
        tokio::fs::write(&thumbnail, b"thumbnail").await.unwrap();
        let completed = CompletedFile {
            id: Uuid::new_v4(),
            job_id: job.id,
            playlist: job.request.playlist.clone(),
            filename: "media.mp4".into(),
            path: media,
            thumbnail_path: Some(thumbnail),
            thumbnail_available: true,
            size_bytes: 5,
            mime_type: "video/mp4".into(),
            title: Some("Fixture".into()),
            browser_playable: true,
            playback: None,
            created_at: Utc::now(),
        };
        storage.insert_completed_file(&completed).await.unwrap();
        (directory, storage, completed)
    }

    #[tokio::test]
    async fn reveal_uses_the_persisted_opaque_file_path() {
        let (_directory, storage, completed) = fixture().await;
        let revealer = Arc::new(RecordingRevealer::default());
        let library = CompletedLibrary::with_revealer(storage, revealer.clone());
        library.reveal_completed(completed.id).await.unwrap();
        assert_eq!(revealer.0.lock().await.as_slice(), [completed.path]);
    }

    #[tokio::test]
    async fn delete_removes_media_thumbnail_and_completed_record() {
        let (_directory, storage, completed) = fixture().await;
        let library = CompletedLibrary::new(storage.clone());
        library.delete_completed(completed.id).await.unwrap();
        assert!(!completed.path.exists());
        assert!(!completed.thumbnail_path.unwrap().exists());
        assert!(
            storage
                .get_completed_file(completed.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(storage.get_job(completed.job_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn playback_progress_is_validated_and_marks_near_end_as_completed() {
        let (_directory, storage, completed) = fixture().await;
        let library = CompletedLibrary::new(storage);
        let progress = library
            .save_playback_progress(
                completed.id,
                PlaybackProgressUpdate {
                    position_seconds: 40.0,
                    duration_seconds: 42.0,
                },
            )
            .await
            .unwrap();
        assert!(progress.completed);
        assert_eq!(progress.position_seconds, 42.0);
        assert!(
            library
                .save_playback_progress(
                    completed.id,
                    PlaybackProgressUpdate {
                        position_seconds: -1.0,
                        duration_seconds: 42.0,
                    },
                )
                .await
                .is_err()
        );
        library.clear_playback_progress(completed.id).await.unwrap();
        assert!(
            library
                .get_completed(completed.id)
                .await
                .unwrap()
                .playback
                .is_none()
        );
    }
}
