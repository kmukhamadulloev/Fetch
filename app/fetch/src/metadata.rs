//! Application service for owned, recoverable background metadata operations.
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use fetch_core::{
    ApplicationEvent, ArtworkUpdate, CompletedFile, EventBus, FetchError, MediaMetadata,
    MetadataSaveState, MetadataSaveStatus, MetadataUpdate,
};
use fetch_storage::Storage;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::metadata_adapter::{MetadataAdapter, io_error, revision};

#[derive(Clone)]
pub struct MetadataService {
    storage: Arc<Storage>,
    adapter: MetadataAdapter,
    events: EventBus,
    pub lock: Arc<Mutex<()>>,
    statuses: Arc<Mutex<HashMap<Uuid, MetadataSaveStatus>>>,
}

#[derive(Serialize, Deserialize)]
struct Journal {
    source: PathBuf,
    backup: PathBuf,
    output: PathBuf,
    artwork: PathBuf,
    old_artwork: Option<PathBuf>,
}

impl MetadataService {
    pub async fn new(
        storage: Arc<Storage>,
        adapter: MetadataAdapter,
        events: EventBus,
    ) -> Result<Self, FetchError> {
        let service = Self {
            storage,
            adapter,
            events,
            lock: Arc::new(Mutex::new(())),
            statuses: Arc::default(),
        };
        service.recover().await?;
        Ok(service)
    }

    async fn file(&self, id: Uuid) -> Result<CompletedFile, FetchError> {
        self.storage
            .get_completed_file(id)
            .await
            .map_err(io_error)?
            .ok_or(FetchError::FileNotFound)
    }

    pub async fn inspect(&self, id: Uuid) -> Result<MediaMetadata, FetchError> {
        let _guard = self.lock.try_lock().map_err(|_| busy())?;
        Ok(self
            .adapter
            .inspect(&self.file(id).await?.path)
            .await?
            .public)
    }

    pub async fn artwork(&self, id: Uuid) -> Result<Vec<u8>, FetchError> {
        let _guard = self.lock.try_lock().map_err(|_| busy())?;
        let file = self.file(id).await?;
        let inspection = self.adapter.inspect(&file.path).await?;
        self.adapter.artwork(&file.path, &inspection).await
    }

    pub async fn status(&self, id: Uuid) -> Result<MetadataSaveStatus, FetchError> {
        self.file(id).await?;
        Ok(self
            .statuses
            .lock()
            .await
            .get(&id)
            .cloned()
            .unwrap_or(MetadataSaveStatus {
                file_id: id,
                operation_id: None,
                state: MetadataSaveState::Idle,
                error: None,
            }))
    }

    pub async fn start(
        &self,
        id: Uuid,
        update: MetadataUpdate,
    ) -> Result<MetadataSaveStatus, FetchError> {
        update.validate()?;
        if self.adapter.shutdown.is_cancelled() {
            return Err(FetchError::InvalidRequest("Fetch is shutting down".into()));
        }
        let guard = self.lock.clone().try_lock_owned().map_err(|_| busy())?;
        self.recover().await?;
        let file = self.file(id).await?;
        if tokio::fs::metadata(&file.path)
            .await
            .map_err(io_error)?
            .permissions()
            .readonly()
        {
            return Err(FetchError::InvalidRequest(
                "This media file is read-only".into(),
            ));
        }
        if revision(&file.path).await? != update.revision {
            return Err(stale());
        }
        let status = MetadataSaveStatus {
            file_id: id,
            operation_id: Some(Uuid::new_v4()),
            state: MetadataSaveState::Saving,
            error: None,
        };
        self.publish(status.clone()).await;
        let service = self.clone();
        let mut final_status = status.clone();
        tokio::spawn(async move {
            let _guard = guard;
            tracing::info!(file_id = %id, "metadata save started");
            match service.save(file, update).await {
                Ok(file) => {
                    service
                        .events
                        .publish(ApplicationEvent::CompletedFileCreated(file));
                    final_status.state = MetadataSaveState::Completed;
                    tracing::info!(file_id = %id, "metadata save completed");
                }
                Err(error) => {
                    final_status.state = MetadataSaveState::Failed;
                    final_status.error = Some(error.public_message());
                    tracing::warn!(file_id = %id, code = ?error.code(), reason = %error.public_message(), "metadata save failed");
                }
            }
            service.publish(final_status).await;
        });
        Ok(status)
    }

    async fn publish(&self, status: MetadataSaveStatus) {
        let mut statuses = self.statuses.lock().await;
        // Only the most recent operation per file is retained; bound session memory.
        if statuses.len() >= 256 {
            statuses.retain(|_, value| value.state == MetadataSaveState::Saving);
        }
        statuses.insert(status.file_id, status.clone());
        self.events.publish(ApplicationEvent::MetadataSaved(status));
    }

    async fn save(
        &self,
        mut file: CompletedFile,
        update: MetadataUpdate,
    ) -> Result<CompletedFile, FetchError> {
        let inspection = self.adapter.inspect(&file.path).await?;
        if inspection.public.revision != update.revision {
            return Err(stale());
        }
        let suffix = Uuid::new_v4();
        let parent = file.path.parent().ok_or(FetchError::FileNotFound)?;
        let journal = Journal {
            source: file.path.clone(),
            backup: parent.join(format!(".fetch-{suffix}.backup")),
            output: parent.join(format!(".fetch-{suffix}.{}", inspection.public.container)),
            artwork: parent.join(format!(".fetch-{suffix}.jpg")),
            old_artwork: file.thumbnail_path.clone(),
        };
        self.storage
            .begin_metadata_edit(file.id, &serde_json::to_string(&journal).map_err(io_error)?)
            .await
            .map_err(io_error)?;
        let result = async {
            let artwork = if let ArtworkUpdate::Replace { data } = &update.artwork {
                self.adapter.prepare_artwork(data, &journal.artwork).await?;
                Some(journal.artwork.as_path())
            } else {
                None
            };
            let output = self
                .adapter
                .write(&file.path, &journal.output, &inspection, &update, artwork)
                .await?;
            if revision(&file.path).await? != update.revision {
                return Err(stale());
            }
            let permissions = tokio::fs::metadata(&file.path)
                .await
                .map_err(io_error)?
                .permissions();
            tokio::fs::set_permissions(&journal.output, permissions)
                .await
                .map_err(io_error)?;
            sync_file(&journal.output).await?;
            if artwork.is_some() {
                sync_file(&journal.artwork).await?;
            }
            // Never truncate the source. The journal restores the backup if a rename
            // or database commit fails, including Windows open-file failures.
            tokio::fs::rename(&file.path, &journal.backup)
                .await
                .map_err(io_error)?;
            tokio::fs::rename(&journal.output, &file.path)
                .await
                .map_err(io_error)?;
            sync_parent(&file.path).await?;
            file.size_bytes = tokio::fs::metadata(&file.path)
                .await
                .map_err(io_error)?
                .len();
            if update.fields.contains_key("title") {
                file.title = output
                    .public
                    .fields
                    .get("title")
                    .filter(|s| !s.is_empty())
                    .cloned();
            }
            match update.artwork {
                ArtworkUpdate::Keep => {}
                ArtworkUpdate::Remove => file.thumbnail_path = None,
                ArtworkUpdate::Replace { .. } => {
                    file.thumbnail_path = Some(journal.artwork.clone())
                }
            }
            file.thumbnail_available = file.thumbnail_path.is_some();
            self.storage
                .finish_metadata_edit(&file)
                .await
                .map_err(io_error)?;
            Ok(file.clone())
        }
        .await;
        // Read the durable commit marker instead of guessing whether a failed DB
        // call committed. Cleanup/recovery is idempotent across process restarts.
        if let Err(error) = self.recover().await {
            tracing::error!(file_id = %file.id, %error, "metadata recovery requires retry");
            return Err(error);
        }
        result
    }

    async fn recover(&self) -> Result<(), FetchError> {
        for (id, json, committed) in self.storage.metadata_edits().await.map_err(io_error)? {
            let journal: Journal = serde_json::from_str(&json).map_err(io_error)?;
            if committed {
                remove(&journal.backup).await?;
                let file = self.file(id).await?;
                if file.thumbnail_path.as_ref() != Some(&journal.artwork) {
                    remove(&journal.artwork).await?;
                }
                if journal.old_artwork != file.thumbnail_path
                    && let Some(old) = journal.old_artwork
                {
                    remove(&old).await?;
                }
            } else {
                if tokio::fs::try_exists(&journal.backup)
                    .await
                    .map_err(io_error)?
                {
                    remove(&journal.source).await?;
                    tokio::fs::rename(&journal.backup, &journal.source)
                        .await
                        .map_err(io_error)?;
                }
                remove(&journal.artwork).await?;
            }
            remove(&journal.output.with_extension("cover")).await?;
            remove(&journal.output).await?;
            sync_parent(&journal.source).await?;
            self.storage
                .clear_metadata_edit(id)
                .await
                .map_err(io_error)?;
        }
        Ok(())
    }
}

async fn sync_parent(path: &std::path::Path) -> Result<(), FetchError> {
    #[cfg(unix)]
    tokio::fs::File::open(path.parent().ok_or(FetchError::FileNotFound)?)
        .await
        .map_err(io_error)?
        .sync_all()
        .await
        .map_err(io_error)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub fn busy() -> FetchError {
    FetchError::Conflict(
        "Another metadata operation is in progress. Try again when it finishes.".into(),
    )
}
fn stale() -> FetchError {
    FetchError::Conflict(
        "This file changed since the editor opened. Reopen it before saving.".into(),
    )
}
async fn sync_file(path: &std::path::Path) -> Result<(), FetchError> {
    tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .await
        .map_err(io_error)?
        .sync_all()
        .await
        .map_err(io_error)
}
async fn remove(path: &std::path::Path) -> Result<(), FetchError> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::tests::fixture;
    use fetch_core::CompletedOperations;
    use fetch_runtime::RuntimePaths;
    use tokio_util::sync::CancellationToken;

    fn adapter(root: &std::path::Path) -> MetadataAdapter {
        MetadataAdapter {
            paths: RuntimePaths::new(root),
            shutdown: CancellationToken::new(),
        }
    }

    #[tokio::test]
    async fn startup_restores_interrupted_replacement_and_cleans_temporary_files() {
        let (dir, storage, file) = fixture().await;
        let journal = Journal {
            source: file.path.clone(),
            backup: dir.path().join("backup"),
            output: dir.path().join("output"),
            artwork: dir.path().join("new-artwork"),
            old_artwork: file.thumbnail_path.clone(),
        };
        storage
            .begin_metadata_edit(file.id, &serde_json::to_string(&journal).unwrap())
            .await
            .unwrap();
        tokio::fs::rename(&file.path, &journal.backup)
            .await
            .unwrap();
        tokio::fs::write(&file.path, b"partial replacement")
            .await
            .unwrap();
        tokio::fs::write(&journal.artwork, b"new thumbnail")
            .await
            .unwrap();
        tokio::fs::write(&journal.output, b"temp").await.unwrap();
        MetadataService::new(storage.clone(), adapter(dir.path()), EventBus::default())
            .await
            .unwrap();
        assert_eq!(tokio::fs::read(&file.path).await.unwrap(), b"media");
        assert!(!journal.backup.exists());
        assert!(!journal.artwork.exists());
        assert!(!journal.output.exists());
        assert!(file.thumbnail_path.unwrap().exists());
        assert!(storage.metadata_edits().await.unwrap().is_empty());
        assert_eq!(
            storage
                .get_completed_file(file.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            file.title
        );
    }

    #[tokio::test]
    async fn committed_recovery_retains_new_media_and_removes_old_backup_and_artwork() {
        let (dir, storage, mut file) = fixture().await;
        let journal = Journal {
            source: file.path.clone(),
            backup: dir.path().join("backup"),
            output: dir.path().join("output"),
            artwork: dir.path().join("new-artwork"),
            old_artwork: file.thumbnail_path.clone(),
        };
        storage
            .begin_metadata_edit(file.id, &serde_json::to_string(&journal).unwrap())
            .await
            .unwrap();
        tokio::fs::rename(&file.path, &journal.backup)
            .await
            .unwrap();
        tokio::fs::write(&file.path, b"new media").await.unwrap();
        tokio::fs::write(&journal.artwork, b"new thumbnail")
            .await
            .unwrap();
        file.title = Some("Saved".into());
        file.size_bytes = 9;
        file.thumbnail_path = Some(journal.artwork.clone());
        storage.finish_metadata_edit(&file).await.unwrap();
        MetadataService::new(storage.clone(), adapter(dir.path()), EventBus::default())
            .await
            .unwrap();
        assert_eq!(tokio::fs::read(&file.path).await.unwrap(), b"new media");
        assert!(!journal.backup.exists());
        assert!(journal.artwork.exists());
        assert!(!journal.old_artwork.unwrap().exists());
        assert!(storage.metadata_edits().await.unwrap().is_empty());
        assert_eq!(
            storage
                .get_completed_file(file.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            file.title
        );
    }

    #[tokio::test]
    async fn rejects_stale_edits_and_serializes_deletion_and_saves() {
        let (dir, storage, file) = fixture().await;
        let service =
            MetadataService::new(storage.clone(), adapter(dir.path()), EventBus::default())
                .await
                .unwrap();
        let update = MetadataUpdate {
            revision: "stale".into(),
            fields: Default::default(),
            artwork: ArtworkUpdate::Keep,
        };
        assert!(matches!(
            service.start(file.id, update).await,
            Err(FetchError::Conflict(_))
        ));
        let library = crate::library::CompletedLibrary::with_metadata(storage, service.clone());
        let guard = service.lock.lock().await;
        assert!(matches!(
            library.delete_completed(file.id).await,
            Err(FetchError::Conflict(_))
        ));
        assert!(file.path.exists());
        drop(guard);
        library.delete_completed(file.id).await.unwrap();
        assert!(!file.path.exists());
    }

    #[tokio::test]
    async fn failed_runtime_operation_reports_failure_and_keeps_original() {
        let (dir, storage, file) = fixture().await;
        let events = EventBus::default();
        let mut receiver = events.subscribe();
        let service = MetadataService::new(storage.clone(), adapter(dir.path()), events)
            .await
            .unwrap();
        let update = MetadataUpdate {
            revision: revision(&file.path).await.unwrap(),
            fields: Default::default(),
            artwork: ArtworkUpdate::Keep,
        };
        let status = service.start(file.id, update).await.unwrap();
        assert_eq!(status.state, MetadataSaveState::Saving);
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if let ApplicationEvent::MetadataSaved(status) = receiver.recv().await.unwrap()
                    && status.state == MetadataSaveState::Failed
                {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(
            service.status(file.id).await.unwrap().state,
            MetadataSaveState::Failed
        );
        assert_eq!(tokio::fs::read(&file.path).await.unwrap(), b"media");
        assert!(storage.metadata_edits().await.unwrap().is_empty());
    }
    #[tokio::test]
    #[ignore = "uses real managed runtime; run explicitly for metadata acceptance"]
    async fn real_managed_metadata_service_updates_library_and_artwork() {
        use base64::Engine;
        let (dir, storage, file) = fixture().await;
        let root = std::env::var_os("FETCH_METADATA_RUNTIME")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("fetch-metadata-runtime-service"));
        let adapter = adapter(&root);
        if !adapter.paths.ffmpeg_executable().is_file()
            || !adapter.paths.ffprobe_executable().is_file()
        {
            fetch_runtime::RuntimeManager::new(adapter.paths.clone())
                .unwrap()
                .install_ffmpeg()
                .await
                .unwrap();
        }
        let output = tokio::process::Command::new(adapter.paths.ffmpeg_executable())
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=64x64:d=1",
                "-c:v",
                "mpeg4",
                "-metadata",
                "title=Original",
            ])
            .arg(&file.path)
            .output()
            .await
            .unwrap();
        assert!(output.status.success());
        let original = tokio::fs::read(&file.path).await.unwrap();
        let cover = dir.path().join("upload.png");
        image::RgbImage::from_pixel(16, 16, image::Rgb([20, 100, 200]))
            .save(&cover)
            .unwrap();
        let data =
            base64::engine::general_purpose::STANDARD.encode(tokio::fs::read(cover).await.unwrap());
        let service = MetadataService::new(storage.clone(), adapter, EventBus::default())
            .await
            .unwrap();
        let library =
            crate::library::CompletedLibrary::with_metadata(storage.clone(), service.clone());
        library
            .save_playback_progress(
                file.id,
                fetch_core::PlaybackProgressUpdate {
                    position_seconds: 0.5,
                    duration_seconds: 1.0,
                },
            )
            .await
            .unwrap();
        async fn wait(service: &MetadataService, id: Uuid) -> MetadataSaveStatus {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                loop {
                    let status = service.status(id).await.unwrap();
                    if status.state != MetadataSaveState::Saving {
                        break status;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap()
        }
        let update = MetadataUpdate {
            revision: service.inspect(file.id).await.unwrap().revision,
            fields: std::collections::BTreeMap::from([("title".into(), "Edited title".into())]),
            artwork: ArtworkUpdate::Replace { data },
        };
        service.start(file.id, update).await.unwrap();
        let status = wait(&service, file.id).await;
        assert_eq!(
            status.state,
            MetadataSaveState::Completed,
            "{:?}",
            status.error
        );
        let saved = storage.get_completed_file(file.id).await.unwrap().unwrap();
        assert_eq!(saved.title.as_deref(), Some("Edited title"));
        assert_eq!(saved.path, file.path);
        assert_eq!(saved.job_id, file.job_id);
        assert_eq!(
            saved.size_bytes,
            tokio::fs::metadata(&file.path).await.unwrap().len()
        );
        assert!(saved.playback.is_some());
        assert!(saved.thumbnail_path.as_ref().unwrap().exists());
        assert!(service.inspect(file.id).await.unwrap().artwork_available);
        assert!(!service.artwork(file.id).await.unwrap().is_empty());
        assert_ne!(tokio::fs::read(&file.path).await.unwrap(), original);
        assert!(storage.metadata_edits().await.unwrap().is_empty());
        let bytes = tokio::fs::read(&file.path).await.unwrap();
        let invalid = MetadataUpdate {
            revision: service.inspect(file.id).await.unwrap().revision,
            fields: Default::default(),
            artwork: ArtworkUpdate::Replace {
                data: "not base64".into(),
            },
        };
        service.start(file.id, invalid).await.unwrap();
        assert_eq!(
            wait(&service, file.id).await.state,
            MetadataSaveState::Failed
        );
        assert_eq!(tokio::fs::read(&file.path).await.unwrap(), bytes);
        let remove = MetadataUpdate {
            revision: service.inspect(file.id).await.unwrap().revision,
            fields: std::collections::BTreeMap::from([("title".into(), String::new())]),
            artwork: ArtworkUpdate::Remove,
        };
        service.start(file.id, remove).await.unwrap();
        let status = wait(&service, file.id).await;
        assert_eq!(
            status.state,
            MetadataSaveState::Completed,
            "{:?}",
            status.error
        );
        let cleared = storage.get_completed_file(file.id).await.unwrap().unwrap();
        assert_eq!(cleared.title, None);
        assert_eq!(cleared.thumbnail_path, None);
        assert!(!saved.thumbnail_path.unwrap().exists());
        assert!(!service.inspect(file.id).await.unwrap().artwork_available);
    }
}
