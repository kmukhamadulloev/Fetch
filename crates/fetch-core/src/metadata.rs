use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::FetchError;

pub const METADATA_FIELDS: &[&str] = &[
    "title",
    "artist",
    "album",
    "album_artist",
    "track",
    "track_total",
    "date",
    "genre",
    "copyright",
    "comment",
    "description",
    "disc",
    "disc_total",
    "composer",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataUpdate {
    pub revision: String,
    /// Omitted keys are preserved; empty strings explicitly clear a tag.
    pub fields: BTreeMap<String, String>,
    pub artwork: ArtworkUpdate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtworkUpdate {
    #[default]
    Keep,
    Remove,
    Replace {
        data: String,
    },
}

impl MetadataUpdate {
    pub fn validate(&self) -> Result<(), FetchError> {
        for (key, value) in &self.fields {
            if !METADATA_FIELDS.contains(&key.as_str())
                || value.len() > 16_384
                || value.contains('\0')
            {
                return Err(FetchError::InvalidRequest(
                    "Invalid metadata field or value".into(),
                ));
            }
            if ["track", "track_total", "disc", "disc_total"].contains(&key.as_str())
                && !value.is_empty()
                && !value.parse::<u16>().is_ok_and(|number| number > 0)
            {
                return Err(FetchError::InvalidRequest(
                    "Track and disc values must be between 1 and 65535".into(),
                ));
            }
        }
        if let ArtworkUpdate::Replace { data } = &self.artwork
            && data.len() > 11_184_812
        {
            return Err(FetchError::InvalidRequest(
                "Artwork must be at most 8 MiB".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaMetadata {
    pub revision: String,
    pub editable: bool,
    pub media_type: String,
    pub container: String,
    pub fields: BTreeMap<String, String>,
    pub supported_fields: Vec<String>,
    pub artwork_available: bool,
    pub artwork_editable: bool,
    pub information: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataSaveState {
    Idle,
    Saving,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataSaveStatus {
    pub file_id: Uuid,
    pub operation_id: Option<Uuid>,
    pub state: MetadataSaveState,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_editable_fields_and_numeric_limits() {
        let update = |key: &str, value: &str| MetadataUpdate {
            revision: "v1".into(),
            fields: BTreeMap::from([(key.into(), value.into())]),
            artwork: ArtworkUpdate::Keep,
        };
        assert!(update("title", "Тест 🎵").validate().is_ok());
        assert!(update("title", "").validate().is_ok());
        assert!(update("track", "65535").validate().is_ok());
        for value in ["-1", "0", "65536", "1/2", "abc"] {
            assert!(update("track", value).validate().is_err());
        }
        assert!(update("arbitrary", "value").validate().is_err());
        assert!(update("title", "a\0b").validate().is_err());
        assert!(
            serde_json::from_str::<MetadataUpdate>(
                r#"{"revision":"v1","fields":{},"artwork":{"action":"keep"},"path":"/tmp/media"}"#
            )
            .is_err()
        );
    }
}
