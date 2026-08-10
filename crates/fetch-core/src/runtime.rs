use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeComponentName {
    YtDlp,
    Ffmpeg,
    Ffprobe,
}

impl RuntimeComponentName {
    pub fn executable_name(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp",
            Self::Ffmpeg => "ffmpeg",
            Self::Ffprobe => "ffprobe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Missing,
    Installing,
    Ready,
    Updating,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeComponent {
    pub name: RuntimeComponentName,
    pub version: Option<String>,
    pub status: RuntimeStatus,
    pub progress_percent: Option<f64>,
    pub error: Option<String>,
}
