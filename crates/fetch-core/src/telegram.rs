use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DownloadMode, FetchError, MediaInfo};

const MAX_ALLOWED_USERS: usize = 64;
const MAX_TOKEN_LENGTH: usize = 512;

#[async_trait::async_trait]
pub trait TelegramOperations: Send + Sync {
    async fn get_integration(&self) -> Result<TelegramIntegration, FetchError>;
    async fn put_settings(
        &self,
        settings: TelegramSettings,
    ) -> Result<TelegramIntegration, FetchError>;
    async fn put_token(&self, token: TelegramToken) -> Result<TelegramIntegration, FetchError>;
    async fn delete_token(&self) -> Result<TelegramIntegration, FetchError>;
    async fn test_connection(&self) -> Result<TelegramIntegration, FetchError>;
}

#[async_trait::async_trait]
pub trait TelegramSecretStore: Send + Sync {
    async fn load(&self) -> Result<Option<TelegramToken>, FetchError>;
    async fn store(&self, token: TelegramToken) -> Result<(), FetchError>;
    async fn delete(&self) -> Result<(), FetchError>;
}

#[async_trait::async_trait]
pub trait TelegramRepository: Send + Sync {
    async fn load_settings(&self) -> Result<TelegramSettings, FetchError>;
    async fn save_settings(&self, settings: &TelegramSettings) -> Result<(), FetchError>;
    async fn polling_offset(&self) -> Result<i64, FetchError>;
    async fn claim_update(&self, update_id: i64) -> Result<bool, FetchError>;
    async fn release_update_claim(&self, update_id: i64) -> Result<(), FetchError>;
    async fn advance_polling_offset(&self, next_offset: i64) -> Result<(), FetchError>;
    async fn save_pending_action(&self, action: &TelegramPendingAction) -> Result<(), FetchError>;
    async fn consume_pending_action(
        &self,
        id: Uuid,
        user_id: i64,
        now: DateTime<Utc>,
    ) -> Result<Option<TelegramPendingAction>, FetchError>;
    async fn associate_job(&self, owner: &TelegramJobOwner) -> Result<(), FetchError>;
    async fn job_owner(&self, job_id: Uuid) -> Result<Option<TelegramJobOwner>, FetchError>;
    async fn owned_job_ids(&self, user_id: i64) -> Result<Vec<Uuid>, FetchError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allowed_user_ids: Vec<i64>,
    #[serde(default = "default_true")]
    pub notify_queued: bool,
    #[serde(default = "default_true")]
    pub notify_completed: bool,
    #[serde(default = "default_true")]
    pub notify_failed: bool,
    #[serde(default)]
    pub privacy_acknowledged: bool,
}

impl Default for TelegramSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_user_ids: Vec::new(),
            notify_queued: true,
            notify_completed: true,
            notify_failed: true,
            privacy_acknowledged: false,
        }
    }
}

impl TelegramSettings {
    pub fn validate(&self) -> Result<(), FetchError> {
        if self.allowed_user_ids.len() > MAX_ALLOWED_USERS {
            return Err(FetchError::InvalidSettings(format!(
                "Telegram allows at most {MAX_ALLOWED_USERS} user IDs"
            )));
        }
        if self.allowed_user_ids.iter().any(|id| *id <= 0) {
            return Err(FetchError::InvalidSettings(
                "Telegram user IDs must be positive integers".into(),
            ));
        }
        let mut unique = self.allowed_user_ids.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() != self.allowed_user_ids.len() {
            return Err(FetchError::InvalidSettings(
                "Telegram user IDs must not contain duplicates".into(),
            ));
        }
        if self.enabled && self.allowed_user_ids.is_empty() {
            return Err(FetchError::InvalidSettings(
                "at least one Telegram user ID is required before enabling the bot".into(),
            ));
        }
        if self.enabled && !self.privacy_acknowledged {
            return Err(FetchError::InvalidSettings(
                "Telegram privacy acknowledgement is required before enabling the bot".into(),
            ));
        }
        Ok(())
    }

    pub fn authorizes(&self, identity: TelegramIdentity) -> bool {
        identity.private_chat
            && identity.user_id == identity.chat_id
            && self.allowed_user_ids.contains(&identity.user_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelegramIdentity {
    pub user_id: i64,
    pub chat_id: i64,
    pub private_chat: bool,
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct TelegramToken(String);

impl TelegramToken {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TelegramToken {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.trim().to_owned();
        if value.is_empty() || value.len() > MAX_TOKEN_LENGTH {
            return Err("the Telegram bot token has an invalid length".into());
        }
        if !value.is_ascii() || value.bytes().any(|byte| byte.is_ascii_control()) {
            return Err("the Telegram bot token contains invalid characters".into());
        }
        Ok(Self(value))
    }
}

impl std::fmt::Debug for TelegramToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TelegramToken(<redacted>)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelegramConnectionState {
    Disabled,
    Connecting,
    Connected,
    BackingOff,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelegramTokenSource {
    Native,
    Environment,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramStatus {
    pub state: TelegramConnectionState,
    pub token_configured: bool,
    pub token_source: TelegramTokenSource,
    pub bot_username: Option<String>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl Default for TelegramStatus {
    fn default() -> Self {
        Self {
            state: TelegramConnectionState::Disabled,
            token_configured: false,
            token_source: TelegramTokenSource::Missing,
            bot_username: None,
            last_success_at: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramIntegration {
    pub settings: TelegramSettings,
    pub status: TelegramStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelegramPendingStage {
    ChooseMode,
    ConfirmPlaylist,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelegramPendingAction {
    pub id: Uuid,
    pub user_id: i64,
    pub chat_id: i64,
    pub source_url: String,
    pub media: MediaInfo,
    pub stage: TelegramPendingStage,
    pub mode: Option<DownloadMode>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

impl TelegramPendingAction {
    pub fn is_available_at(&self, now: DateTime<Utc>) -> bool {
        self.consumed_at.is_none() && now < self.expires_at
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramJobOwner {
    pub job_id: Uuid,
    pub user_id: i64,
    pub chat_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelegramNotificationKind {
    Queued,
    Completed,
    Failed,
    Stopped,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_defaults_are_safe_and_backward_compatible() {
        let settings: TelegramSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, TelegramSettings::default());
        settings.validate().unwrap();
    }

    #[test]
    fn enabled_settings_require_unique_users_and_privacy_acknowledgement() {
        for allowed_user_ids in [vec![], vec![42, 42], vec![-1]] {
            let settings = TelegramSettings {
                enabled: true,
                allowed_user_ids,
                privacy_acknowledged: true,
                ..TelegramSettings::default()
            };
            assert!(settings.validate().is_err());
        }
        let missing_acknowledgement = TelegramSettings {
            enabled: true,
            allowed_user_ids: vec![42],
            ..TelegramSettings::default()
        };
        assert!(missing_acknowledgement.validate().is_err());
    }

    #[test]
    fn authorization_requires_allowlisted_matching_private_chat_identity() {
        let settings = TelegramSettings {
            allowed_user_ids: vec![42],
            ..TelegramSettings::default()
        };
        assert!(settings.authorizes(TelegramIdentity {
            user_id: 42,
            chat_id: 42,
            private_chat: true,
        }));
        assert!(!settings.authorizes(TelegramIdentity {
            user_id: 42,
            chat_id: -100,
            private_chat: false,
        }));
        assert!(!settings.authorizes(TelegramIdentity {
            user_id: 7,
            chat_id: 7,
            private_chat: true,
        }));
    }

    #[test]
    fn token_debug_never_exposes_the_secret() {
        let secret = "123456:token-value";
        let token = TelegramToken::try_from(secret.to_owned()).unwrap();
        assert_eq!(token.expose(), secret);
        assert!(!format!("{token:?}").contains(secret));
        assert!(TelegramToken::try_from(String::from("\n")).is_err());
    }
}
