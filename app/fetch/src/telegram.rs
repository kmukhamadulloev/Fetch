use std::env;

use fetch_core::{FetchError, TelegramSecretStore, TelegramToken};

const CREDENTIAL_SERVICE: &str = "io.fetch.downloader";
const CREDENTIAL_USER: &str = "telegram-bot-token";
pub const TOKEN_ENVIRONMENT_VARIABLE: &str = "FETCH_TELEGRAM_BOT_TOKEN";

#[derive(Debug, Clone, Default)]
pub struct NativeTelegramSecretStore;

#[async_trait::async_trait]
impl TelegramSecretStore for NativeTelegramSecretStore {
    async fn load(&self) -> Result<Option<TelegramToken>, FetchError> {
        tokio::task::spawn_blocking(|| {
            let entry = credential_entry()?;
            match entry.get_password() {
                Ok(value) => TelegramToken::try_from(value).map(Some).map_err(|_| {
                    FetchError::Internal(
                        "the Telegram token in the native credential store is invalid".into(),
                    )
                }),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(error) => Err(credential_error("read", error)),
            }
        })
        .await
        .map_err(|error| FetchError::Internal(format!("credential task failed: {error}")))?
    }

    async fn store(&self, token: TelegramToken) -> Result<(), FetchError> {
        tokio::task::spawn_blocking(move || {
            credential_entry()?
                .set_password(token.expose())
                .map_err(|error| credential_error("save", error))
        })
        .await
        .map_err(|error| FetchError::Internal(format!("credential task failed: {error}")))?
    }

    async fn delete(&self) -> Result<(), FetchError> {
        tokio::task::spawn_blocking(|| {
            let entry = credential_entry()?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(credential_error("delete", error)),
            }
        })
        .await
        .map_err(|error| FetchError::Internal(format!("credential task failed: {error}")))?
    }
}

pub fn token_from_environment() -> Result<Option<TelegramToken>, FetchError> {
    match env::var(TOKEN_ENVIRONMENT_VARIABLE) {
        Ok(value) => TelegramToken::try_from(value).map(Some).map_err(|message| {
            FetchError::InvalidSettings(format!(
                "{TOKEN_ENVIRONMENT_VARIABLE} is invalid: {message}"
            ))
        }),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(FetchError::InvalidSettings(format!(
            "{TOKEN_ENVIRONMENT_VARIABLE} must contain valid Unicode"
        ))),
    }
}

fn credential_entry() -> Result<keyring::Entry, FetchError> {
    keyring::Entry::new(CREDENTIAL_SERVICE, CREDENTIAL_USER)
        .map_err(|error| credential_error("open", error))
}

fn credential_error(operation: &str, error: keyring::Error) -> FetchError {
    tracing::warn!(
        subsystem = "telegram",
        operation,
        error = %error,
        "native Telegram credential operation failed"
    );
    FetchError::InvalidSettings(format!(
        "could not {operation} the Telegram token in the native credential store; use {TOKEN_ENVIRONMENT_VARIABLE} on headless systems"
    ))
}
