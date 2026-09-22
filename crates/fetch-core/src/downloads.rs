use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{FetchError, TelegramStatus, YtDlpJsRuntime};

#[async_trait::async_trait]
pub trait DownloadOperations: Send + Sync {
    async fn create(&self, request: DownloadRequest) -> Result<DownloadJob, FetchError>;
    async fn list(&self) -> Result<Vec<DownloadJob>, FetchError>;
    async fn get(&self, id: Uuid) -> Result<DownloadJob, FetchError>;
    async fn stop(&self, id: Uuid) -> Result<DownloadJob, FetchError>;
    async fn resume(&self, id: Uuid) -> Result<DownloadJob, FetchError>;
    async fn retry(&self, id: Uuid) -> Result<DownloadJob, FetchError>;
    async fn delete(&self, id: Uuid) -> Result<(), FetchError>;
    async fn update_defaults(
        &self,
        download_directory: PathBuf,
        concurrent_downloads: u8,
        ytdlp_js_runtime: YtDlpJsRuntime,
    ) -> Result<(), FetchError>;
}

#[async_trait::async_trait]
pub trait CompletedOperations: Send + Sync {
    async fn metadata(&self, _id: Uuid) -> Result<crate::MediaMetadata, FetchError> {
        Err(FetchError::InvalidRequest(
            "Metadata editing is unavailable".into(),
        ))
    }
    async fn metadata_artwork(&self, _id: Uuid) -> Result<Vec<u8>, FetchError> {
        Err(FetchError::FileNotFound)
    }
    async fn update_metadata(
        &self,
        _id: Uuid,
        _update: crate::MetadataUpdate,
    ) -> Result<crate::MetadataSaveStatus, FetchError> {
        Err(FetchError::InvalidRequest(
            "Metadata editing is unavailable".into(),
        ))
    }
    async fn metadata_status(&self, _id: Uuid) -> Result<crate::MetadataSaveStatus, FetchError> {
        Err(FetchError::NotFound)
    }
    async fn list_completed(&self) -> Result<Vec<CompletedFile>, FetchError>;
    async fn get_completed(&self, id: Uuid) -> Result<CompletedFile, FetchError>;
    async fn reveal_completed(&self, id: Uuid) -> Result<(), FetchError>;
    async fn delete_completed(&self, id: Uuid) -> Result<(), FetchError>;
    async fn save_playback_progress(
        &self,
        id: Uuid,
        update: PlaybackProgressUpdate,
    ) -> Result<PlaybackProgress, FetchError>;
    async fn clear_playback_progress(&self, id: Uuid) -> Result<(), FetchError>;
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ApplicationEvent {
    #[serde(rename = "download.created")]
    DownloadCreated(DownloadJob),
    #[serde(rename = "download.progress")]
    DownloadProgress(DownloadJob),
    #[serde(rename = "download.postprocessing")]
    DownloadPostprocessing(DownloadJob),
    #[serde(rename = "download.completed")]
    DownloadCompleted(DownloadJob),
    #[serde(rename = "download.failed")]
    DownloadFailed(DownloadJob),
    #[serde(rename = "download.stopped")]
    DownloadStopped(DownloadJob),
    #[serde(rename = "library.completed")]
    CompletedFileCreated(CompletedFile),
    #[serde(rename = "library.progress")]
    PlaybackProgressUpdated(PlaybackProgress),
    #[serde(rename = "library.progress-cleared")]
    PlaybackProgressCleared(Uuid),
    #[serde(rename = "library.metadata")]
    MetadataSaved(crate::MetadataSaveStatus),
    #[serde(rename = "telegram.status")]
    TelegramStatusUpdated(TelegramStatus),
}

impl ApplicationEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::DownloadCreated(_) => "download.created",
            Self::DownloadProgress(_) => "download.progress",
            Self::DownloadPostprocessing(_) => "download.postprocessing",
            Self::DownloadCompleted(_) => "download.completed",
            Self::DownloadFailed(_) => "download.failed",
            Self::DownloadStopped(_) => "download.stopped",
            Self::CompletedFileCreated(_) => "library.completed",
            Self::PlaybackProgressUpdated(_) => "library.progress",
            Self::PlaybackProgressCleared(_) => "library.progress-cleared",
            Self::TelegramStatusUpdated(_) => "telegram.status",
            Self::MetadataSaved(_) => "library.metadata",
        }
    }

    pub fn job(&self) -> Option<&DownloadJob> {
        match self {
            Self::DownloadCreated(job)
            | Self::DownloadProgress(job)
            | Self::DownloadPostprocessing(job)
            | Self::DownloadCompleted(job)
            | Self::DownloadFailed(job)
            | Self::DownloadStopped(job) => Some(job),
            Self::CompletedFileCreated(_)
            | Self::PlaybackProgressUpdated(_)
            | Self::PlaybackProgressCleared(_)
            | Self::TelegramStatusUpdated(_)
            | Self::MetadataSaved(_) => None,
        }
    }

    pub fn completed_file(&self) -> Option<&CompletedFile> {
        match self {
            Self::CompletedFileCreated(file) => Some(file),
            _ => None,
        }
    }

    pub fn playback_progress(&self) -> Option<&PlaybackProgress> {
        match self {
            Self::PlaybackProgressUpdated(progress) => Some(progress),
            _ => None,
        }
    }

    pub fn cleared_playback_file(&self) -> Option<&Uuid> {
        match self {
            Self::PlaybackProgressCleared(id) => Some(id),
            _ => None,
        }
    }

    pub fn telegram_status(&self) -> Option<&TelegramStatus> {
        match self {
            Self::TelegramStatusUpdated(status) => Some(status),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct EventBus {
    sender: tokio::sync::broadcast::Sender<ApplicationEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(256);
        Self { sender }
    }
}

impl EventBus {
    pub fn publish(&self, event: ApplicationEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ApplicationEvent> {
        self.sender.subscribe()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadMode {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistContext {
    pub id: String,
    pub title: String,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub url: String,
    pub title: Option<String>,
    pub duration_seconds: Option<f64>,
    pub mode: DownloadMode,
    pub format_id: Option<String>,
    pub quality: Option<String>,
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    #[serde(default)]
    pub embed_metadata: bool,
    #[serde(default)]
    pub embed_thumbnail: bool,
    #[serde(default)]
    pub subtitles: bool,
    #[serde(default)]
    pub playlist: Option<PlaylistContext>,
    pub output_directory: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadStatus {
    Created,
    Analyzing,
    Ready,
    Queued,
    Downloading,
    Postprocessing,
    Completed,
    Failed,
    Stopped,
}

impl DownloadStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Analyzing)
                | (Self::Created, Self::Queued)
                | (Self::Analyzing, Self::Ready)
                | (Self::Analyzing, Self::Failed)
                | (Self::Ready, Self::Queued)
                | (Self::Queued, Self::Downloading)
                | (Self::Queued, Self::Stopped)
                | (Self::Queued, Self::Failed)
                | (Self::Downloading, Self::Postprocessing)
                | (Self::Downloading, Self::Completed)
                | (Self::Downloading, Self::Failed)
                | (Self::Downloading, Self::Stopped)
                | (Self::Postprocessing, Self::Completed)
                | (Self::Postprocessing, Self::Failed)
                | (Self::Postprocessing, Self::Stopped)
                | (Self::Stopped, Self::Queued)
                | (Self::Failed, Self::Queued)
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub progress_percent: Option<f64>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_second: Option<u64>,
    pub eta_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: Uuid,
    #[serde(flatten)]
    pub request: DownloadRequest,
    pub status: DownloadStatus,
    #[serde(flatten)]
    pub progress: DownloadProgress,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DownloadJob {
    pub fn new(request: DownloadRequest) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            request,
            status: DownloadStatus::Created,
            progress: DownloadProgress::default(),
            error_code: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn transition(&mut self, next: DownloadStatus) -> Result<(), FetchError> {
        if !self.status.can_transition_to(next) {
            return Err(FetchError::InvalidTransition {
                from: self.status,
                to: next,
            });
        }
        self.status = next;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletedFile {
    pub id: Uuid,
    pub job_id: Uuid,
    pub playlist: Option<PlaylistContext>,
    pub filename: String,
    #[serde(skip_serializing)]
    pub path: PathBuf,
    #[serde(skip_serializing)]
    pub thumbnail_path: Option<PathBuf>,
    pub thumbnail_available: bool,
    pub size_bytes: u64,
    pub mime_type: String,
    pub title: Option<String>,
    pub browser_playable: bool,
    pub playback: Option<PlaybackProgress>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackProgress {
    pub file_id: Uuid,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub completed: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct PlaybackProgressUpdate {
    pub position_seconds: f64,
    pub duration_seconds: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> DownloadRequest {
        DownloadRequest {
            url: "https://example.test/video".into(),
            title: None,
            duration_seconds: None,
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
            output_directory: None,
        }
    }

    #[test]
    fn enforces_state_machine_and_recovery() {
        let mut job = DownloadJob::new(request());
        job.transition(DownloadStatus::Queued).unwrap();
        job.transition(DownloadStatus::Downloading).unwrap();
        job.transition(DownloadStatus::Stopped).unwrap();
        job.transition(DownloadStatus::Queued).unwrap();
        assert!(job.transition(DownloadStatus::Completed).is_err());
    }

    #[test]
    fn exposes_telegram_status_as_a_typed_realtime_event() {
        let status = TelegramStatus::default();
        let event = ApplicationEvent::TelegramStatusUpdated(status.clone());

        assert_eq!(event.event_name(), "telegram.status");
        assert_eq!(event.telegram_status(), Some(&status));
        assert!(event.job().is_none());
    }
}
