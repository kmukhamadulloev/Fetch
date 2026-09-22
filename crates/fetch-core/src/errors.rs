use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    UnsupportedUrl,
    AuthenticationRequired,
    GeoRestricted,
    MediaUnavailable,
    RuntimeMissing,
    RuntimeCorrupt,
    ProcessFailed,
    InvalidSettings,
    InvalidRequest,
    Conflict,
    OutputDirectoryUnavailable,
    FileNotFound,
    NetworkDenied,
    LocalClientRequired,
    InvalidTransition,
    NotFound,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error: ErrorBody {
                code,
                message: message.into(),
                details: None,
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("the URL is not supported by the installed yt-dlp runtime")]
    UnsupportedUrl,
    #[error("the media requires authentication")]
    AuthenticationRequired,
    #[error("the media is not available in this region")]
    GeoRestricted,
    #[error("the media is unavailable")]
    MediaUnavailable,
    #[error("required runtime component {0} is missing")]
    RuntimeMissing(String),
    #[error("runtime component {0} is corrupt or unhealthy")]
    RuntimeCorrupt(String),
    #[error("external process failed: {summary}")]
    ProcessFailed { summary: String, details: String },
    #[error("invalid settings: {0}")]
    InvalidSettings(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("output directory is unavailable: {0}")]
    OutputDirectoryUnavailable(String),
    #[error("file was not found")]
    FileNotFound,
    #[error("network client is not allowed")]
    NetworkDenied,
    #[error("this operation is available only on the device running Fetch")]
    LocalClientRequired,
    #[error("invalid job transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: crate::DownloadStatus,
        to: crate::DownloadStatus,
    },
    #[error("resource was not found")]
    NotFound,
    #[error("internal application error: {0}")]
    Internal(String),
}

impl FetchError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::UnsupportedUrl => ErrorCode::UnsupportedUrl,
            Self::AuthenticationRequired => ErrorCode::AuthenticationRequired,
            Self::GeoRestricted => ErrorCode::GeoRestricted,
            Self::MediaUnavailable => ErrorCode::MediaUnavailable,
            Self::RuntimeMissing(_) => ErrorCode::RuntimeMissing,
            Self::RuntimeCorrupt(_) => ErrorCode::RuntimeCorrupt,
            Self::ProcessFailed { .. } => ErrorCode::ProcessFailed,
            Self::InvalidSettings(_) => ErrorCode::InvalidSettings,
            Self::InvalidRequest(_) => ErrorCode::InvalidRequest,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::OutputDirectoryUnavailable(_) => ErrorCode::OutputDirectoryUnavailable,
            Self::FileNotFound => ErrorCode::FileNotFound,
            Self::NetworkDenied => ErrorCode::NetworkDenied,
            Self::LocalClientRequired => ErrorCode::LocalClientRequired,
            Self::InvalidTransition { .. } => ErrorCode::InvalidTransition,
            Self::NotFound => ErrorCode::NotFound,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    pub fn public_message(&self) -> String {
        match self {
            Self::ProcessFailed { summary, .. } => summary.clone(),
            Self::Internal(_) => "An internal application error occurred".to_owned(),
            _ => self.to_string(),
        }
    }

    pub fn diagnostic_details(&self) -> Option<&str> {
        match self {
            Self::ProcessFailed { details, .. } if !details.is_empty() => Some(details),
            _ => None,
        }
    }
}
