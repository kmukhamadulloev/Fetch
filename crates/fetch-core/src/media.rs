use serde::{Deserialize, Serialize};

use crate::FetchError;

#[async_trait::async_trait]
pub trait MediaAnalysis: Send + Sync {
    async fn analyze(&self, url: &str) -> Result<MediaInfo, FetchError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Media,
    Playlist,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaInfo {
    pub kind: MediaKind,
    pub id: Option<String>,
    pub extractor: Option<String>,
    pub title: String,
    pub webpage_url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub thumbnail_url: Option<String>,
    pub playlist_count: Option<u64>,
    pub entries: Vec<PlaylistEntry>,
    pub formats: Vec<MediaFormat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaFormat {
    pub id: String,
    pub label: String,
    pub extension: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub bitrate_kbps: Option<f64>,
    pub filesize_bytes: Option<u64>,
    pub has_video: bool,
    pub has_audio: bool,
}
