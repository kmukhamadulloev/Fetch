//! Validated local export requests and persistent processing lifecycle.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::FetchError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingKind {
    Conversion,
    Edit,
    Metadata,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}
impl ProcessingState {
    pub fn terminal(self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Mp4,
    Mkv,
    M4a,
    Mp3,
    Flac,
    Wav,
}
impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::M4a => "m4a",
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Wav => "wav",
        }
    }
    pub fn video(self) -> bool {
        matches!(self, Self::Mp4 | Self::Mkv)
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportQuality {
    Compact,
    #[default]
    Balanced,
    High,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resize {
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickEdits {
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
    #[serde(default)]
    pub rotate: u16,
    #[serde(default)]
    pub mute: bool,
    pub crop: Option<Crop>,
    pub resize: Option<Resize>,
    pub volume: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
    pub revision: String,
    pub filename: String,
    pub format: OutputFormat,
    #[serde(default)]
    pub quality: ExportQuality,
    #[serde(default)]
    pub stream_copy: bool,
    pub edits: Option<QuickEdits>,
    #[serde(default)]
    pub acknowledge_omissions: bool,
}
impl ExportRequest {
    pub fn kind(&self) -> ProcessingKind {
        if self.edits.is_some() {
            ProcessingKind::Edit
        } else {
            ProcessingKind::Conversion
        }
    }
    pub fn validate(&self) -> Result<(), FetchError> {
        let invalid = || FetchError::InvalidRequest("Invalid export options".into());
        let stem = self.filename.trim();
        let reserved = stem
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if self.revision.is_empty()
            || self.revision.len() > 128
            || stem.is_empty()
            || stem.len() > 180
            || stem != self.filename
            || stem.ends_with('.')
            || stem
                .chars()
                .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
            || [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ]
            .contains(&reserved.as_str())
            || stem.starts_with('.')
        {
            return Err(invalid());
        }
        if self.stream_copy && (self.edits.is_some() || self.format == OutputFormat::Wav) {
            return Err(invalid());
        }
        if let Some(e) = &self.edits
            && (![0, 90, 180, 270].contains(&e.rotate)
                || e == &QuickEdits::default()
                || e.start_seconds.is_some_and(|v| !v.is_finite() || v < 0.)
                || e.end_seconds
                    .is_some_and(|v| !v.is_finite() || v <= e.start_seconds.unwrap_or(0.))
                || e.volume
                    .is_some_and(|v| !v.is_finite() || !(0. ..=4.).contains(&v) || e.mute)
                || e.crop.as_ref().is_some_and(|c| {
                    c.width == 0
                        || c.height == 0
                        || c.x.checked_add(c.width).is_none()
                        || c.y.checked_add(c.height).is_none()
                })
                || e.resize.as_ref().is_some_and(|r| {
                    r.width == 0
                        || r.height == 0
                        || r.width > 7680
                        || r.height > 7680
                        || r.width % 2 != 0
                        || r.height % 2 != 0
                }))
        {
            return Err(invalid());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessingJob {
    pub id: Uuid,
    pub kind: ProcessingKind,
    pub source_file_id: Uuid,
    pub output_file_id: Option<Uuid>,
    pub title: String,
    pub state: ProcessingState,
    pub stage: String,
    pub progress_percent: Option<f64>,
    pub eta_seconds: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
impl ProcessingJob {
    pub fn new(kind: ProcessingKind, source_file_id: Uuid, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            kind,
            source_file_id,
            output_file_id: None,
            title,
            state: ProcessingState::Queued,
            stage: "queued".into(),
            progress_percent: None,
            eta_seconds: None,
            created_at: now,
            started_at: None,
            updated_at: now,
            finished_at: None,
            error: None,
        }
    }

    /// Retrying creates a new job; terminal attempts are immutable history.
    pub fn transition(&mut self, next: ProcessingState) -> Result<(), FetchError> {
        use ProcessingState::*;
        let allowed = matches!(
            (self.state, next),
            (Queued, Running | Failed | Cancelled)
                | (Running, Completed | Failed | Cancelled | Interrupted)
        );
        if !allowed || (self.kind == ProcessingKind::Metadata && next == Cancelled) {
            return Err(FetchError::Conflict(
                "This processing action is no longer available".into(),
            ));
        }
        let now = Utc::now();
        self.state = next;
        self.updated_at = now;
        if next == Running {
            self.started_at = Some(now);
        }
        if next.terminal() {
            self.finished_at = Some(now);
            self.eta_seconds = None;
        }
        if next == Completed {
            self.progress_percent = Some(100.);
        }
        Ok(())
    }

    pub fn can_cancel(&self) -> bool {
        self.kind != ProcessingKind::Metadata && !self.state.terminal()
    }
    pub fn can_retry(&self) -> bool {
        self.kind != ProcessingKind::Metadata
            && matches!(
                self.state,
                ProcessingState::Failed | ProcessingState::Cancelled | ProcessingState::Interrupted
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> ExportRequest {
        ExportRequest {
            revision: "1-2".into(),
            filename: "Export 🎵".into(),
            format: OutputFormat::Mp4,
            quality: ExportQuality::Balanced,
            stream_copy: false,
            edits: None,
            acknowledge_omissions: false,
        }
    }
    #[test]
    fn state_machine_keeps_terminal_history_and_honest_controls() {
        let mut job = ProcessingJob::new(ProcessingKind::Conversion, Uuid::new_v4(), "Test".into());
        assert!(job.can_cancel());
        assert!(!job.can_retry());
        assert!(job.transition(ProcessingState::Completed).is_err());
        job.transition(ProcessingState::Running).unwrap();
        assert!(job.started_at.is_some());
        job.transition(ProcessingState::Interrupted).unwrap();
        assert!(job.can_retry());
        assert!(!job.can_cancel());
        assert!(job.finished_at.is_some());
        assert_eq!(job.progress_percent, None);
        assert!(job.transition(ProcessingState::Running).is_err());
        let mut metadata =
            ProcessingJob::new(ProcessingKind::Metadata, Uuid::new_v4(), "Metadata".into());
        assert!(!metadata.can_cancel());
        assert!(metadata.transition(ProcessingState::Cancelled).is_err());
        metadata.transition(ProcessingState::Running).unwrap();
        metadata.transition(ProcessingState::Completed).unwrap();
        assert_eq!(metadata.progress_percent, Some(100.));
        assert!(!metadata.can_retry());
    }

    #[test]
    fn rejects_unsafe_names_and_unknown_arguments() {
        assert!(request().validate().is_ok());
        for name in [
            "../x", "a/b", "a\\b", "CON.mp4", "LPT9", ".hidden", "a:", "a\0b", " a", "a.", "",
        ] {
            let mut r = request();
            r.filename = name.into();
            assert!(r.validate().is_err(), "{name}");
        }
        let mut json = serde_json::to_value(request()).unwrap();
        json["args"] = serde_json::json!(["-y"]);
        assert!(serde_json::from_value::<ExportRequest>(json).is_err());
    }
    #[test]
    fn validates_edit_ranges_and_conflicting_options() {
        let mut r = request();
        r.edits = Some(QuickEdits {
            rotate: 90,
            ..Default::default()
        });
        assert!(r.validate().is_ok());
        r.stream_copy = true;
        assert!(r.validate().is_err());
        r.stream_copy = false;
        for e in [
            QuickEdits::default(),
            QuickEdits {
                rotate: 45,
                ..Default::default()
            },
            QuickEdits {
                start_seconds: Some(f64::NAN),
                ..Default::default()
            },
            QuickEdits {
                start_seconds: Some(3.),
                end_seconds: Some(2.),
                ..Default::default()
            },
            QuickEdits {
                mute: true,
                volume: Some(2.),
                ..Default::default()
            },
            QuickEdits {
                crop: Some(Crop {
                    x: u32::MAX,
                    y: 0,
                    width: 2,
                    height: 2,
                }),
                ..Default::default()
            },
            QuickEdits {
                resize: Some(Resize {
                    width: 63,
                    height: 48,
                }),
                ..Default::default()
            },
        ] {
            r.edits = Some(e);
            assert!(r.validate().is_err());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingCapabilities {
    pub revision: String,
    pub duration_seconds: f64,
    pub video: bool,
    pub audio: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub formats: Vec<OutputFormat>,
    pub copy_formats: Vec<OutputFormat>,
    /// Stable notice codes, translated by the client before acknowledgement.
    pub notices: Vec<String>,
}
