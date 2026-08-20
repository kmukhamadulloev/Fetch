use std::{
    collections::{HashMap, VecDeque},
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Utc;
use fetch_core::{
    AppStatus, ApplicationEvent, DownloadMode, DownloadOperations, DownloadRequest, DownloadStatus,
    EventBus, FetchError, MediaAnalysis, MediaKind, PlaylistContext, ProxyMode, ProxyPolicy,
    TelegramConnectionState, TelegramIdentity, TelegramIntegration, TelegramJobOwner,
    TelegramOperations, TelegramPendingAction, TelegramPendingStage, TelegramSecretStore,
    TelegramSettings, TelegramStatus, TelegramToken, TelegramTokenSource,
};
use fetch_storage::Storage;
use fetch_telegram::{
    Backoff, BotApiClient, CallbackAction, IncomingMessage, InlineKeyboardButton,
    InlineKeyboardMarkup, SendMessage, TelegramApi, TelegramError, Update,
};
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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

#[derive(Clone)]
pub struct TelegramBotManager {
    inner: Arc<ManagerInner>,
}

struct ManagerInner {
    storage: Arc<Storage>,
    secrets: NativeTelegramSecretStore,
    media: Arc<dyn MediaAnalysis>,
    downloads: Arc<dyn DownloadOperations>,
    events: EventBus,
    app_status: fetch_core::StatusService,
    proxy: ProxyPolicy,
    status: RwLock<TelegramStatus>,
    poller: tokio::sync::Mutex<Option<RunningTask>>,
    notifier: tokio::sync::Mutex<Option<RunningTask>>,
    command_windows: tokio::sync::Mutex<HashMap<i64, VecDeque<Instant>>>,
}

struct RunningTask {
    cancellation: CancellationToken,
    handle: JoinHandle<()>,
}

impl TelegramBotManager {
    pub fn new(
        storage: Arc<Storage>,
        media: Arc<dyn MediaAnalysis>,
        downloads: Arc<dyn DownloadOperations>,
        events: EventBus,
        app_status: fetch_core::StatusService,
        proxy: ProxyPolicy,
    ) -> Self {
        Self {
            inner: Arc::new(ManagerInner {
                storage,
                secrets: NativeTelegramSecretStore,
                media,
                downloads,
                events,
                app_status,
                proxy,
                status: RwLock::new(TelegramStatus::default()),
                poller: tokio::sync::Mutex::new(None),
                notifier: tokio::sync::Mutex::new(None),
                command_windows: tokio::sync::Mutex::new(HashMap::new()),
            }),
        }
    }

    pub async fn start(&self) {
        self.restart_poller().await;
        let mut notifier = self.inner.notifier.lock().await;
        if notifier.is_none() {
            let cancellation = CancellationToken::new();
            let inner = self.inner.clone();
            let task_cancellation = cancellation.clone();
            let handle = tokio::spawn(async move {
                run_notifications(inner, task_cancellation).await;
            });
            *notifier = Some(RunningTask {
                cancellation,
                handle,
            });
        }
    }

    pub async fn shutdown(&self) {
        stop_task(&self.inner.poller).await;
        stop_task(&self.inner.notifier).await;
    }

    async fn effective_token(
        &self,
    ) -> Result<(Option<TelegramToken>, TelegramTokenSource), FetchError> {
        if let Some(token) = token_from_environment()? {
            return Ok((Some(token), TelegramTokenSource::Environment));
        }
        let token = self.inner.secrets.load().await?;
        let source = if token.is_some() {
            TelegramTokenSource::Native
        } else {
            TelegramTokenSource::Missing
        };
        Ok((token, source))
    }

    async fn restart_poller(&self) {
        stop_task(&self.inner.poller).await;
        let settings = match self.inner.storage.load_telegram_settings().await {
            Ok(settings) => settings,
            Err(error) => {
                self.set_error("could not load Telegram settings", &error.to_string())
                    .await;
                return;
            }
        };
        if !settings.enabled {
            let current = self.inner.status.read().await.clone();
            publish_status(
                &self.inner,
                TelegramStatus {
                    token_configured: current.token_configured,
                    token_source: current.token_source,
                    ..TelegramStatus::default()
                },
            )
            .await;
            return;
        }
        let (token, source) = match self.effective_token().await {
            Ok(value) => value,
            Err(error) => {
                self.set_error(&error.public_message(), &error.to_string())
                    .await;
                return;
            }
        };
        let Some(token) = token else {
            publish_status(
                &self.inner,
                TelegramStatus {
                    state: TelegramConnectionState::Error,
                    token_configured: false,
                    token_source: TelegramTokenSource::Missing,
                    error: Some("Add a Telegram bot token before enabling the integration".into()),
                    ..TelegramStatus::default()
                },
            )
            .await;
            return;
        };
        let proxy_mode = proxy_mode_name(self.inner.proxy.current().mode);
        tracing::info!(
            subsystem = "telegram",
            proxy_mode,
            "Telegram bot connecting"
        );
        let _ = self
            .inner
            .storage
            .append_log(
                "info",
                "telegram",
                "Telegram bot connecting",
                Some(&format!("proxy_mode: {proxy_mode}")),
            )
            .await;
        publish_status(
            &self.inner,
            TelegramStatus {
                state: TelegramConnectionState::Connecting,
                token_configured: true,
                token_source: source,
                ..TelegramStatus::default()
            },
        )
        .await;
        let cancellation = CancellationToken::new();
        let task_cancellation = cancellation.clone();
        let inner = self.inner.clone();
        let handle = tokio::spawn(async move {
            run_poller(inner, token, settings, source, task_cancellation).await;
        });
        *self.inner.poller.lock().await = Some(RunningTask {
            cancellation,
            handle,
        });
    }

    async fn set_error(&self, public: &str, diagnostic: &str) {
        tracing::warn!(
            subsystem = "telegram",
            error = diagnostic,
            "Telegram integration error"
        );
        let _ = self
            .inner
            .storage
            .append_log("error", "telegram", public, Some(diagnostic))
            .await;
        update_status(&self.inner, |status| {
            status.state = TelegramConnectionState::Error;
            status.error = Some(public.to_owned());
        })
        .await;
    }

    async fn set_telegram_error(&self, error: &TelegramError, source: TelegramTokenSource) {
        let public = error.to_string();
        self.set_error(&public, &public).await;
        update_status(&self.inner, |status| {
            status.token_configured = true;
            status.token_source = source;
        })
        .await;
    }
}

#[async_trait::async_trait]
impl TelegramOperations for TelegramBotManager {
    async fn get_integration(&self) -> Result<TelegramIntegration, FetchError> {
        if self.inner.status.read().await.state == TelegramConnectionState::Disabled {
            let (token, source) = self.effective_token().await?;
            let mut status = self.inner.status.write().await;
            status.token_configured = token.is_some();
            status.token_source = source;
        }
        Ok(TelegramIntegration {
            settings: self
                .inner
                .storage
                .load_telegram_settings()
                .await
                .map_err(storage_error)?,
            status: self.inner.status.read().await.clone(),
        })
    }

    async fn put_settings(
        &self,
        settings: TelegramSettings,
    ) -> Result<TelegramIntegration, FetchError> {
        settings.validate()?;
        self.inner
            .storage
            .save_telegram_settings(&settings)
            .await
            .map_err(storage_error)?;
        let _ = self
            .inner
            .storage
            .append_log(
                "info",
                "telegram",
                if settings.enabled {
                    "Telegram integration enabled"
                } else {
                    "Telegram integration disabled"
                },
                None,
            )
            .await;
        self.restart_poller().await;
        self.get_integration().await
    }

    async fn put_token(&self, token: TelegramToken) -> Result<TelegramIntegration, FetchError> {
        self.inner.secrets.store(token).await?;
        update_status(&self.inner, |status| {
            status.token_configured = true;
            status.token_source = if token_from_environment()?.is_some() {
                TelegramTokenSource::Environment
            } else {
                TelegramTokenSource::Native
            };
            Ok::<(), FetchError>(())
        })
        .await?;
        self.restart_poller().await;
        self.get_integration().await
    }

    async fn delete_token(&self) -> Result<TelegramIntegration, FetchError> {
        self.inner.secrets.delete().await?;
        update_status(&self.inner, |status| {
            status.token_configured = token_from_environment()?.is_some();
            status.token_source = if status.token_configured {
                TelegramTokenSource::Environment
            } else {
                TelegramTokenSource::Missing
            };
            Ok::<(), FetchError>(())
        })
        .await?;
        self.restart_poller().await;
        self.get_integration().await
    }

    async fn test_connection(&self) -> Result<TelegramIntegration, FetchError> {
        let (token, source) = self.effective_token().await?;
        let token = token.ok_or_else(|| {
            FetchError::InvalidSettings("Add a Telegram bot token before testing".into())
        })?;
        let proxy = self.inner.proxy.current();
        let proxy_mode = proxy_mode_name(proxy.mode);
        tracing::info!(
            subsystem = "telegram",
            proxy_mode,
            "Testing Telegram bot connection"
        );
        let _ = self
            .inner
            .storage
            .append_log(
                "info",
                "telegram",
                "Testing Telegram bot connection",
                Some(&format!("proxy_mode: {proxy_mode}")),
            )
            .await;
        let client = match BotApiClient::new(token, &proxy) {
            Ok(client) => client,
            Err(error) => {
                self.set_telegram_error(&error, source).await;
                return Err(FetchError::InvalidSettings(error.to_string()));
            }
        };
        match client.get_me().await {
            Ok(bot) => {
                update_status(&self.inner, |status| {
                    status.token_configured = true;
                    status.token_source = source;
                    status.bot_username = bot.username;
                    status.last_success_at = Some(Utc::now());
                    status.error = None;
                    if status.state == TelegramConnectionState::Error {
                        status.state = TelegramConnectionState::Disabled;
                    }
                })
                .await;
                tracing::info!(
                    subsystem = "telegram",
                    proxy_mode,
                    "Telegram bot connection test succeeded"
                );
                let _ = self
                    .inner
                    .storage
                    .append_log(
                        "info",
                        "telegram",
                        "Telegram bot connection test succeeded",
                        Some(&format!("proxy_mode: {proxy_mode}")),
                    )
                    .await;
            }
            Err(error) => {
                self.set_telegram_error(&error, source).await;
                return Err(FetchError::InvalidSettings(error.to_string()));
            }
        }
        if self
            .inner
            .storage
            .load_telegram_settings()
            .await
            .map_err(storage_error)?
            .enabled
        {
            self.restart_poller().await;
        }
        self.get_integration().await
    }
}

async fn stop_task(slot: &tokio::sync::Mutex<Option<RunningTask>>) {
    if let Some(task) = slot.lock().await.take() {
        task.cancellation.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(3), task.handle).await;
    }
}

async fn publish_status(inner: &ManagerInner, status: TelegramStatus) {
    *inner.status.write().await = status.clone();
    inner
        .events
        .publish(ApplicationEvent::TelegramStatusUpdated(status));
}

async fn update_status<F, R>(inner: &ManagerInner, update: F) -> R
where
    F: FnOnce(&mut TelegramStatus) -> R,
{
    let mut status = inner.status.write().await;
    let result = update(&mut status);
    let event = status.clone();
    drop(status);
    inner
        .events
        .publish(ApplicationEvent::TelegramStatusUpdated(event));
    result
}

async fn run_poller(
    inner: Arc<ManagerInner>,
    token: TelegramToken,
    settings: TelegramSettings,
    source: TelegramTokenSource,
    cancellation: CancellationToken,
) {
    let mut proxy_updates = inner.proxy.subscribe();
    loop {
        let proxy = proxy_updates.borrow_and_update().clone();
        let proxy_mode = proxy_mode_name(proxy.mode);
        let client = match BotApiClient::new(token.clone(), &proxy) {
            Ok(client) => client,
            Err(error) => {
                record_poller_error(&inner, &error, source).await;
                return;
            }
        };
        let identity = tokio::select! {
            _ = cancellation.cancelled() => return,
            changed = proxy_updates.changed() => {
                if changed.is_err() {
                    return;
                }
                let mode = { proxy_updates.borrow().mode };
                record_proxy_reconnect(&inner, mode).await;
                update_status(&inner, |status| {
                    status.state = TelegramConnectionState::Connecting;
                    status.error = None;
                }).await;
                continue;
            }
            result = client.get_me() => result,
        };
        match identity {
            Ok(bot) => {
                publish_status(
                    &inner,
                    TelegramStatus {
                        state: TelegramConnectionState::Connected,
                        token_configured: true,
                        token_source: source,
                        bot_username: bot.username,
                        last_success_at: Some(Utc::now()),
                        error: None,
                    },
                )
                .await;
                let _ = inner
                    .storage
                    .append_log(
                        "info",
                        "telegram",
                        "Telegram bot connected",
                        Some(&format!("proxy_mode: {proxy_mode}")),
                    )
                    .await;
            }
            Err(error) => {
                record_poller_error(&inner, &error, source).await;
                return;
            }
        }
        let mut backoff = Backoff::default();
        loop {
            let offset = match inner.storage.telegram_polling_offset().await {
                Ok(offset) => offset,
                Err(error) => {
                    tracing::error!(subsystem = "telegram", %error, "could not load polling offset");
                    return;
                }
            };
            let poll_result = tokio::select! {
                _ = cancellation.cancelled() => return,
                changed = proxy_updates.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    let mode = { proxy_updates.borrow().mode };
                    record_proxy_reconnect(&inner, mode).await;
                    None
                }
                result = client.get_updates(offset, Duration::from_secs(25), &cancellation) => Some(result),
            };
            let Some(poll_result) = poll_result else {
                update_status(&inner, |status| {
                    status.state = TelegramConnectionState::Connecting;
                    status.error = None;
                })
                .await;
                break;
            };
            match poll_result {
                Ok(updates) => {
                    backoff.reset();
                    for update in updates {
                        if cancellation.is_cancelled() {
                            return;
                        }
                        let next_offset = update.update_id.saturating_add(1);
                        match inner.storage.claim_telegram_update(update.update_id).await {
                            Ok(true) => {
                                if let Err(error) =
                                    handle_update(&inner, &client, &settings, update).await
                                {
                                    tracing::warn!(subsystem = "telegram", error = %error, "Telegram update handling failed");
                                    if let Err(release_error) = inner
                                        .storage
                                        .release_telegram_update_claim(
                                            next_offset.saturating_sub(1),
                                        )
                                        .await
                                    {
                                        tracing::error!(subsystem = "telegram", error = %release_error, "could not release failed Telegram update claim");
                                        return;
                                    }
                                    tokio::select! {
                                        _ = cancellation.cancelled() => return,
                                        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                                    }
                                    break;
                                }
                            }
                            Ok(false) => {}
                            Err(error) => {
                                tracing::error!(subsystem = "telegram", %error, "could not deduplicate Telegram update");
                                return;
                            }
                        }
                        if let Err(error) = inner
                            .storage
                            .complete_telegram_update(next_offset.saturating_sub(1), next_offset)
                            .await
                        {
                            tracing::error!(subsystem = "telegram", %error, "could not persist Telegram polling offset");
                            return;
                        }
                    }
                    update_status(&inner, |status| {
                        status.state = TelegramConnectionState::Connected;
                        status.last_success_at = Some(Utc::now());
                        status.error = None;
                    })
                    .await;
                }
                Err(TelegramError::Cancelled) => return,
                Err(error) if error.retryable() => {
                    let delay = backoff.next_delay(&error);
                    tracing::warn!(
                        subsystem = "telegram",
                        error = %error,
                        retry_delay_seconds = delay.as_secs(),
                        proxy_mode,
                        "Telegram polling interrupted; retry scheduled"
                    );
                    let _ = inner
                        .storage
                        .append_log(
                            "warn",
                            "telegram",
                            "Telegram polling interrupted; retry scheduled",
                            Some(&format!(
                                "cause: {error}\nretry_delay_seconds: {}\nproxy_mode: {proxy_mode}",
                                delay.as_secs()
                            )),
                        )
                        .await;
                    update_status(&inner, |status| {
                        status.state = TelegramConnectionState::BackingOff;
                        status.error = Some(error.to_string());
                    })
                    .await;
                    let proxy_changed = tokio::select! {
                        _ = cancellation.cancelled() => return,
                        changed = proxy_updates.changed() => {
                            if changed.is_err() {
                                return;
                            }
                            true
                        }
                        _ = tokio::time::sleep(delay) => false
                    };
                    if proxy_changed {
                        let mode = { proxy_updates.borrow().mode };
                        record_proxy_reconnect(&inner, mode).await;
                        update_status(&inner, |status| {
                            status.state = TelegramConnectionState::Connecting;
                            status.error = None;
                        })
                        .await;
                        break;
                    }
                }
                Err(error) => {
                    record_poller_error(&inner, &error, source).await;
                    return;
                }
            }
        }
    }
}

async fn record_proxy_reconnect(inner: &ManagerInner, mode: ProxyMode) {
    let proxy_mode = proxy_mode_name(mode);
    tracing::info!(
        subsystem = "telegram",
        proxy_mode,
        "Telegram outbound route changed; reconnecting"
    );
    let _ = inner
        .storage
        .append_log(
            "info",
            "telegram",
            "Telegram outbound route changed; reconnecting",
            Some(&format!("proxy_mode: {proxy_mode}")),
        )
        .await;
}

async fn record_poller_error(
    inner: &ManagerInner,
    error: &TelegramError,
    source: TelegramTokenSource,
) {
    let message = error.to_string();
    tracing::warn!(subsystem = "telegram", error = %message, "Telegram poller stopped");
    let _ = inner
        .storage
        .append_log("error", "telegram", &message, None)
        .await;
    publish_status(
        inner,
        TelegramStatus {
            state: TelegramConnectionState::Error,
            token_configured: true,
            token_source: source,
            error: Some(message),
            ..TelegramStatus::default()
        },
    )
    .await;
}

async fn handle_update(
    inner: &ManagerInner,
    client: &dyn TelegramApi,
    settings: &TelegramSettings,
    update: Update,
) -> Result<(), FetchError> {
    if let Some(message) = update.message {
        let Some(user) = message.from else {
            return Ok(());
        };
        let identity = TelegramIdentity {
            user_id: user.id,
            chat_id: message.chat.id,
            private_chat: message.chat.kind == "private",
        };
        if !settings.authorizes(identity) {
            return Ok(());
        }
        if !rate_limit_allows(inner, identity.user_id).await {
            return Ok(());
        }
        let action = message
            .text
            .as_deref()
            .map(IncomingMessage::parse)
            .unwrap_or(IncomingMessage::Unsupported);
        let action_kind = incoming_kind(&action);
        handle_message(inner, client, identity, action).await?;
        let _ = inner
            .storage
            .append_log(
                "info",
                "telegram",
                "Authorized Telegram message processed",
                Some(action_kind),
            )
            .await;
    } else if let Some(callback) = update.callback_query {
        let Some(message) = callback.message else {
            return Ok(());
        };
        let identity = TelegramIdentity {
            user_id: callback.from.id,
            chat_id: message.chat.id,
            private_chat: message.chat.kind == "private",
        };
        if !settings.authorizes(identity) {
            return Ok(());
        }
        if !rate_limit_allows(inner, identity.user_id).await {
            return Ok(());
        }
        if let Some(action) = callback.data.as_deref().and_then(CallbackAction::parse) {
            handle_callback(inner, client, identity, action).await?;
            telegram(client.answer_callback_query(&callback.id).await)?;
            let _ = inner
                .storage
                .append_log(
                    "info",
                    "telegram",
                    "Authorized Telegram action processed",
                    Some(callback_kind(action)),
                )
                .await;
        }
    }
    Ok(())
}

async fn rate_limit_allows(inner: &ManagerInner, user_id: i64) -> bool {
    const WINDOW: Duration = Duration::from_secs(60);
    const MAX_ACTIONS: usize = 20;
    let now = Instant::now();
    let mut windows = inner.command_windows.lock().await;
    let actions = windows.entry(user_id).or_default();
    while actions
        .front()
        .is_some_and(|at| now.duration_since(*at) >= WINDOW)
    {
        actions.pop_front();
    }
    if actions.len() >= MAX_ACTIONS {
        return false;
    }
    actions.push_back(now);
    true
}

async fn handle_message(
    inner: &ManagerInner,
    client: &dyn TelegramApi,
    identity: TelegramIdentity,
    action: IncomingMessage,
) -> Result<(), FetchError> {
    match action {
        IncomingMessage::Start | IncomingMessage::Help => send_text(client, identity.chat_id, "Fetch can queue downloads on your host. Send a media URL, then choose Video or Audio. Commands: /status, /downloads.", None).await,
        IncomingMessage::Status => {
            let AppStatus { runtime_ready, storage_ready, .. } = inner.app_status.status();
            let text = format!("Fetch is running. Runtime: {}. Storage: {}.", ready_word(runtime_ready), ready_word(storage_ready));
            send_text(client, identity.chat_id, &text, None).await
        }
        IncomingMessage::Downloads => send_downloads(inner, client, identity).await,
        IncomingMessage::SourceUrl(url) => analyze_source(inner, client, identity, url).await,
        IncomingMessage::Unsupported => send_text(client, identity.chat_id, "Send an http(s) media URL or use /help.", None).await,
    }
}

async fn analyze_source(
    inner: &ManagerInner,
    client: &dyn TelegramApi,
    identity: TelegramIdentity,
    url: String,
) -> Result<(), FetchError> {
    send_text(client, identity.chat_id, "Analyzing media…", None).await?;
    let media = match inner.media.analyze(&url).await {
        Ok(media) => media,
        Err(error) => {
            return send_text(
                client,
                identity.chat_id,
                &format!("Could not analyze media: {}", error.public_message()),
                None,
            )
            .await;
        }
    };
    let action = TelegramPendingAction {
        id: Uuid::new_v4(),
        user_id: identity.user_id,
        chat_id: identity.chat_id,
        source_url: url,
        media,
        stage: TelegramPendingStage::ChooseMode,
        mode: None,
        expires_at: Utc::now() + chrono::Duration::minutes(10),
        consumed_at: None,
    };
    inner
        .storage
        .save_telegram_pending_action(&action)
        .await
        .map_err(storage_error)?;
    let keyboard = InlineKeyboardMarkup {
        inline_keyboard: vec![vec![
            button("Video", CallbackAction::Video(action.id)),
            button("Audio", CallbackAction::Audio(action.id)),
            button("Cancel", CallbackAction::Cancel(action.id)),
        ]],
    };
    let count = action
        .media
        .playlist_count
        .or_else(|| u64::try_from(action.media.entries.len()).ok());
    let summary = if action.media.kind == MediaKind::Playlist {
        format!(
            "{}\nPlaylist · {} items",
            action.media.title,
            count.unwrap_or(0)
        )
    } else {
        action.media.title.clone()
    };
    send_text(client, identity.chat_id, &summary, Some(&keyboard)).await
}

async fn handle_callback(
    inner: &ManagerInner,
    client: &dyn TelegramApi,
    identity: TelegramIdentity,
    callback: CallbackAction,
) -> Result<(), FetchError> {
    match callback {
        CallbackAction::Video(id) | CallbackAction::Audio(id) => {
            let mode = if matches!(callback, CallbackAction::Video(_)) {
                DownloadMode::Video
            } else {
                DownloadMode::Audio
            };
            let Some(mut action) = inner
                .storage
                .consume_telegram_pending_action(id, identity.user_id, Utc::now())
                .await
                .map_err(storage_error)?
            else {
                return send_text(
                    client,
                    identity.chat_id,
                    "This action expired or was already used.",
                    None,
                )
                .await;
            };
            if action.stage != TelegramPendingStage::ChooseMode {
                return Ok(());
            }
            if action.media.kind == MediaKind::Playlist {
                action.id = Uuid::new_v4();
                action.stage = TelegramPendingStage::ConfirmPlaylist;
                action.mode = Some(mode);
                action.consumed_at = None;
                action.expires_at = Utc::now() + chrono::Duration::minutes(10);
                inner
                    .storage
                    .save_telegram_pending_action(&action)
                    .await
                    .map_err(storage_error)?;
                let keyboard = InlineKeyboardMarkup {
                    inline_keyboard: vec![vec![
                        button(
                            "Download playlist",
                            CallbackAction::ConfirmPlaylist(action.id),
                        ),
                        button("Cancel", CallbackAction::Cancel(action.id)),
                    ]],
                };
                send_text(
                    client,
                    identity.chat_id,
                    "Confirm downloading every available playlist item.",
                    Some(&keyboard),
                )
                .await
            } else {
                enqueue_action(inner, client, identity, action, mode).await
            }
        }
        CallbackAction::ConfirmPlaylist(id) => {
            let Some(action) = inner
                .storage
                .consume_telegram_pending_action(id, identity.user_id, Utc::now())
                .await
                .map_err(storage_error)?
            else {
                return send_text(
                    client,
                    identity.chat_id,
                    "This action expired or was already used.",
                    None,
                )
                .await;
            };
            if action.stage != TelegramPendingStage::ConfirmPlaylist {
                return Ok(());
            }
            let mode = action.mode.ok_or_else(|| {
                FetchError::Internal("Telegram playlist confirmation had no mode".into())
            })?;
            enqueue_action(inner, client, identity, action, mode).await
        }
        CallbackAction::Cancel(id) => {
            let _ = inner
                .storage
                .consume_telegram_pending_action(id, identity.user_id, Utc::now())
                .await
                .map_err(storage_error)?;
            send_text(client, identity.chat_id, "Cancelled.", None).await
        }
        CallbackAction::Stop(job_id) => {
            let owner = inner
                .storage
                .telegram_job_owner(job_id)
                .await
                .map_err(storage_error)?;
            if owner.as_ref().is_some_and(|owner| {
                owner.user_id == identity.user_id && owner.chat_id == identity.chat_id
            }) {
                inner.downloads.stop(job_id).await?;
                send_text(client, identity.chat_id, "Stopping download…", None).await
            } else {
                send_text(
                    client,
                    identity.chat_id,
                    "That download is not available to this Telegram user.",
                    None,
                )
                .await
            }
        }
    }
}

async fn enqueue_action(
    inner: &ManagerInner,
    client: &dyn TelegramApi,
    identity: TelegramIdentity,
    action: TelegramPendingAction,
    mode: DownloadMode,
) -> Result<(), FetchError> {
    let requests = requests_for_media(&action, mode)?;
    let total = requests.len();
    for request in requests {
        let job = inner.downloads.create(request).await?;
        inner
            .storage
            .associate_telegram_job(&TelegramJobOwner {
                job_id: job.id,
                user_id: identity.user_id,
                chat_id: identity.chat_id,
            })
            .await
            .map_err(storage_error)?;
    }
    let settings = inner
        .storage
        .load_telegram_settings()
        .await
        .map_err(storage_error)?;
    if settings.notify_queued {
        send_text(
            client,
            identity.chat_id,
            &format!(
                "Queued {total} download{}.",
                if total == 1 { "" } else { "s" }
            ),
            None,
        )
        .await?;
    }
    Ok(())
}

fn requests_for_media(
    action: &TelegramPendingAction,
    mode: DownloadMode,
) -> Result<Vec<DownloadRequest>, FetchError> {
    let base = |url: String, title: String, duration_seconds, playlist| DownloadRequest {
        url,
        title: Some(title),
        duration_seconds,
        mode,
        format_id: None,
        quality: None,
        container: None,
        video_codec: None,
        audio_codec: None,
        embed_metadata: false,
        embed_thumbnail: false,
        subtitles: false,
        playlist,
        output_directory: None,
    };
    if action.media.kind != MediaKind::Playlist {
        return Ok(vec![base(
            action
                .media
                .webpage_url
                .clone()
                .unwrap_or_else(|| action.source_url.clone()),
            action.media.title.clone(),
            action.media.duration_seconds,
            None,
        )]);
    }
    let playlist_id = action
        .media
        .id
        .clone()
        .unwrap_or_else(|| action.id.to_string());
    let requests = action
        .media
        .entries
        .iter()
        .filter_map(|entry| entry.url.clone().map(|url| (entry, url)))
        .enumerate()
        .map(|(index, (entry, url))| {
            base(
                url,
                entry.title.clone(),
                entry.duration_seconds,
                Some(PlaylistContext {
                    id: playlist_id.clone(),
                    title: action.media.title.clone(),
                    index: u32::try_from(index + 1).unwrap_or(u32::MAX),
                }),
            )
        })
        .collect::<Vec<_>>();
    if requests.is_empty() {
        Err(FetchError::InvalidRequest(
            "The playlist did not contain downloadable item URLs".into(),
        ))
    } else {
        Ok(requests)
    }
}

async fn send_downloads(
    inner: &ManagerInner,
    client: &dyn TelegramApi,
    identity: TelegramIdentity,
) -> Result<(), FetchError> {
    let ids = inner
        .storage
        .telegram_owned_job_ids(identity.user_id)
        .await
        .map_err(storage_error)?;
    let mut lines = Vec::new();
    let mut buttons = Vec::new();
    for id in ids.into_iter().take(10) {
        if let Ok(job) = inner.downloads.get(id).await {
            lines.push(format!(
                "{} — {:?}",
                job.request.title.as_deref().unwrap_or("Untitled"),
                job.status
            ));
            if matches!(
                job.status,
                DownloadStatus::Queued
                    | DownloadStatus::Downloading
                    | DownloadStatus::Postprocessing
            ) {
                buttons.push(vec![button(
                    &format!("Stop {}", short_title(job.request.title.as_deref())),
                    CallbackAction::Stop(job.id),
                )]);
            }
        }
    }
    if lines.is_empty() {
        return send_text(
            client,
            identity.chat_id,
            "No downloads were created by this Telegram user.",
            None,
        )
        .await;
    }
    let keyboard = InlineKeyboardMarkup {
        inline_keyboard: buttons,
    };
    send_text(
        client,
        identity.chat_id,
        &lines.join("\n"),
        (!keyboard.inline_keyboard.is_empty()).then_some(&keyboard),
    )
    .await
}

async fn run_notifications(inner: Arc<ManagerInner>, cancellation: CancellationToken) {
    let mut receiver = inner.events.subscribe();
    loop {
        let event = tokio::select! {
            _ = cancellation.cancelled() => return,
            event = receiver.recv() => match event {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!(subsystem = "telegram", skipped_events = count, "Telegram notification listener lagged");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            }
        };
        if !matches!(
            &event,
            ApplicationEvent::DownloadCompleted(_)
                | ApplicationEvent::DownloadFailed(_)
                | ApplicationEvent::DownloadStopped(_)
        ) {
            continue;
        }
        let Ok((Some(token), _)) = effective_token_for(&inner.secrets).await else {
            continue;
        };
        let proxy = inner.proxy.current();
        let proxy_mode = proxy_mode_name(proxy.mode);
        let client = match BotApiClient::new(token, &proxy) {
            Ok(client) => client,
            Err(error) => {
                let _ = inner
                    .storage
                    .append_log(
                        "warn",
                        "telegram",
                        "Telegram notification client could not be configured",
                        Some(&format!("cause: {error}\nproxy_mode: {proxy_mode}")),
                    )
                    .await;
                continue;
            }
        };
        if let Err(error) = deliver_notification(&inner, &client, &event).await {
            tracing::warn!(subsystem = "telegram", error = %error.public_message(), "could not send Telegram terminal notification");
            let _ = inner
                .storage
                .append_log(
                    "warn",
                    "telegram",
                    "Telegram terminal notification failed",
                    Some(&format!(
                        "cause: {}\nproxy_mode: {proxy_mode}",
                        error.public_message()
                    )),
                )
                .await;
        }
    }
}

fn proxy_mode_name(mode: ProxyMode) -> &'static str {
    match mode {
        ProxyMode::System => "system",
        ProxyMode::Direct => "direct",
        ProxyMode::Custom => "custom",
    }
}

async fn deliver_notification(
    inner: &ManagerInner,
    client: &dyn TelegramApi,
    event: &ApplicationEvent,
) -> Result<(), FetchError> {
    let (job, completed) = match event {
        ApplicationEvent::DownloadCompleted(job) => (job, true),
        ApplicationEvent::DownloadFailed(job) | ApplicationEvent::DownloadStopped(job) => {
            (job, false)
        }
        _ => return Ok(()),
    };
    let Some(owner) = inner
        .storage
        .telegram_job_owner(job.id)
        .await
        .map_err(storage_error)?
    else {
        return Ok(());
    };
    let settings = inner
        .storage
        .load_telegram_settings()
        .await
        .map_err(storage_error)?;
    let notifications_enabled = if completed {
        settings.notify_completed
    } else {
        settings.notify_failed
    };
    if !settings.enabled || !notifications_enabled {
        return Ok(());
    }
    let state = match job.status {
        DownloadStatus::Completed => "Completed",
        DownloadStatus::Failed => "Failed",
        DownloadStatus::Stopped => "Stopped",
        _ => return Ok(()),
    };
    let text = format!(
        "{state}: {}",
        job.request.title.as_deref().unwrap_or("Untitled download")
    );
    send_text(client, owner.chat_id, &text, None).await
}

async fn effective_token_for(
    store: &NativeTelegramSecretStore,
) -> Result<(Option<TelegramToken>, TelegramTokenSource), FetchError> {
    if let Some(token) = token_from_environment()? {
        return Ok((Some(token), TelegramTokenSource::Environment));
    }
    let token = store.load().await?;
    let source = if token.is_some() {
        TelegramTokenSource::Native
    } else {
        TelegramTokenSource::Missing
    };
    Ok((token, source))
}

async fn send_text(
    client: &dyn TelegramApi,
    chat_id: i64,
    text: &str,
    keyboard: Option<&InlineKeyboardMarkup>,
) -> Result<(), FetchError> {
    telegram(
        client
            .send_message(SendMessage {
                chat_id,
                text,
                reply_markup: keyboard,
            })
            .await,
    )?;
    Ok(())
}

fn telegram<T>(result: Result<T, TelegramError>) -> Result<T, FetchError> {
    result.map_err(|error| FetchError::Internal(error.to_string()))
}

fn storage_error(error: fetch_storage::StorageError) -> FetchError {
    FetchError::Internal(error.to_string())
}

fn button(text: &str, action: CallbackAction) -> InlineKeyboardButton {
    InlineKeyboardButton {
        text: text.into(),
        callback_data: action.encode(),
    }
}

fn ready_word(value: bool) -> &'static str {
    if value { "ready" } else { "not ready" }
}

fn incoming_kind(action: &IncomingMessage) -> &'static str {
    match action {
        IncomingMessage::Start => "command=start",
        IncomingMessage::Help => "command=help",
        IncomingMessage::Status => "command=status",
        IncomingMessage::Downloads => "command=downloads",
        IncomingMessage::SourceUrl(_) => "command=analyze_url",
        IncomingMessage::Unsupported => "command=unsupported",
    }
}

fn callback_kind(action: CallbackAction) -> &'static str {
    match action {
        CallbackAction::Video(_) => "action=video",
        CallbackAction::Audio(_) => "action=audio",
        CallbackAction::ConfirmPlaylist(_) => "action=confirm_playlist",
        CallbackAction::Cancel(_) => "action=cancel",
        CallbackAction::Stop(_) => "action=stop",
    }
}

fn short_title(title: Option<&str>) -> String {
    let title = title.unwrap_or("download");
    let mut characters = title.chars();
    let short = characters.by_ref().take(24).collect::<String>();
    if characters.next().is_some() {
        format!("{short}…")
    } else {
        short
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fetch_core::{DownloadJob, MediaInfo, PlaylistEntry};
    use fetch_telegram::{BotUser, Chat, Message, User};
    use tokio::sync::Mutex;

    use super::*;

    struct FakeMedia;
    struct FakeDownloads {
        storage: Arc<Storage>,
        jobs: Mutex<HashMap<Uuid, DownloadJob>>,
        stopped: Mutex<Vec<Uuid>>,
    }

    #[derive(Clone)]
    struct SentMessage {
        chat_id: i64,
        text: String,
        keyboard: Option<InlineKeyboardMarkup>,
    }

    #[derive(Default)]
    struct FakeTelegramApi {
        messages: Mutex<Vec<SentMessage>>,
    }

    #[async_trait::async_trait]
    impl MediaAnalysis for FakeMedia {
        async fn analyze(&self, url: &str) -> Result<MediaInfo, FetchError> {
            Ok(MediaInfo {
                kind: MediaKind::Media,
                id: Some("fixture".into()),
                extractor: Some("fixture".into()),
                title: "Fixture media".into(),
                webpage_url: Some(url.into()),
                duration_seconds: Some(42.0),
                thumbnail_url: None,
                playlist_count: None,
                entries: Vec::new(),
                formats: Vec::new(),
            })
        }
    }

    #[async_trait::async_trait]
    impl DownloadOperations for FakeDownloads {
        async fn create(&self, request: DownloadRequest) -> Result<DownloadJob, FetchError> {
            let job = DownloadJob::new(request);
            self.storage.insert_job(&job).await.map_err(storage_error)?;
            self.jobs.lock().await.insert(job.id, job.clone());
            Ok(job)
        }
        async fn list(&self) -> Result<Vec<DownloadJob>, FetchError> {
            Ok(self.jobs.lock().await.values().cloned().collect())
        }
        async fn get(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
            self.jobs
                .lock()
                .await
                .get(&id)
                .cloned()
                .ok_or(FetchError::NotFound)
        }
        async fn stop(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
            let job = self.get(id).await?;
            self.stopped.lock().await.push(id);
            Ok(job)
        }
        async fn resume(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
            self.get(id).await
        }
        async fn retry(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
            self.get(id).await
        }
        async fn delete(&self, _id: Uuid) -> Result<(), FetchError> {
            Err(FetchError::NotFound)
        }
        async fn update_defaults(
            &self,
            _download_directory: PathBuf,
            _concurrent_downloads: u8,
            _ytdlp_js_runtime: fetch_core::YtDlpJsRuntime,
        ) -> Result<(), FetchError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl TelegramApi for FakeTelegramApi {
        async fn get_me(&self) -> Result<BotUser, TelegramError> {
            unreachable!()
        }
        async fn get_updates(
            &self,
            _offset: i64,
            _long_poll_timeout: Duration,
            _cancellation: &CancellationToken,
        ) -> Result<Vec<Update>, TelegramError> {
            unreachable!()
        }
        async fn send_message(&self, request: SendMessage<'_>) -> Result<Message, TelegramError> {
            self.messages.lock().await.push(SentMessage {
                chat_id: request.chat_id,
                text: request.text.into(),
                keyboard: request.reply_markup.cloned(),
            });
            Ok(Message {
                message_id: 1,
                from: None,
                chat: Chat {
                    id: request.chat_id,
                    kind: "private".into(),
                },
                text: Some(request.text.into()),
            })
        }
        async fn answer_callback_query(
            &self,
            _callback_query_id: &str,
        ) -> Result<(), TelegramError> {
            Ok(())
        }
    }

    async fn test_inner() -> (ManagerInner, Arc<FakeDownloads>) {
        let storage = Arc::new(
            Storage::open(std::path::Path::new(":memory:"))
                .await
                .unwrap(),
        );
        let downloads = Arc::new(FakeDownloads {
            storage: storage.clone(),
            jobs: Mutex::new(HashMap::new()),
            stopped: Mutex::new(Vec::new()),
        });
        let inner = ManagerInner {
            storage,
            secrets: NativeTelegramSecretStore,
            media: Arc::new(FakeMedia),
            downloads: downloads.clone(),
            events: EventBus::default(),
            app_status: fetch_core::StatusService::new("0.1.4", true, true),
            proxy: ProxyPolicy::new(fetch_core::ProxySettings::default()).unwrap(),
            status: RwLock::new(TelegramStatus::default()),
            poller: tokio::sync::Mutex::new(None),
            notifier: tokio::sync::Mutex::new(None),
            command_windows: tokio::sync::Mutex::new(HashMap::new()),
        };
        (inner, downloads)
    }

    fn message_update(update_id: i64, user_id: i64, text: &str) -> Update {
        Update {
            update_id,
            message: Some(Message {
                message_id: 1,
                from: Some(User {
                    id: user_id,
                    is_bot: false,
                    username: None,
                }),
                chat: Chat {
                    id: user_id,
                    kind: "private".into(),
                },
                text: Some(text.into()),
            }),
            callback_query: None,
        }
    }

    #[tokio::test]
    async fn status_changes_publish_realtime_events() {
        let (inner, _) = test_inner().await;
        let mut events = inner.events.subscribe();

        update_status(&inner, |status| {
            status.state = TelegramConnectionState::Connected;
            status.bot_username = Some("fetch_bot".into());
        })
        .await;

        let event = events.recv().await.unwrap();
        assert_eq!(event.event_name(), "telegram.status");
        assert_eq!(
            event.telegram_status().unwrap().state,
            TelegramConnectionState::Connected
        );
        assert_eq!(
            event.telegram_status().unwrap().bot_username.as_deref(),
            Some("fetch_bot")
        );
    }

    #[tokio::test]
    async fn proxy_reconnects_and_failures_are_retained_without_endpoints() {
        let (inner, _) = test_inner().await;
        inner
            .proxy
            .replace(fetch_core::ProxySettings {
                mode: ProxyMode::Custom,
                url: Some("http://private.proxy:8080".into()),
            })
            .unwrap();

        record_proxy_reconnect(&inner, ProxyMode::Custom).await;
        record_poller_error(&inner, &TelegramError::Network, TelegramTokenSource::Native).await;

        let logs = inner.storage.list_logs(10).await.unwrap();
        assert!(logs.iter().any(|entry| {
            entry.subsystem == "telegram" && entry.message == "Telegram is unreachable"
        }));
        assert!(logs.iter().any(|entry| {
            entry.message == "Telegram outbound route changed; reconnecting"
                && entry.details.as_deref() == Some("proxy_mode: custom")
        }));
        assert!(!format!("{logs:?}").contains("private.proxy"));
    }

    fn callback_update(update_id: i64, user_id: i64, data: String) -> Update {
        Update {
            update_id,
            message: None,
            callback_query: Some(fetch_telegram::CallbackQuery {
                id: format!("callback-{update_id}"),
                from: User {
                    id: user_id,
                    is_bot: false,
                    username: None,
                },
                message: Some(Message {
                    message_id: 1,
                    from: None,
                    chat: Chat {
                        id: user_id,
                        kind: "private".into(),
                    },
                    text: None,
                }),
                data: Some(data),
            }),
        }
    }

    #[tokio::test]
    async fn ignores_unauthorized_updates_and_answers_authorized_status() {
        let (inner, _) = test_inner().await;
        let client = FakeTelegramApi::default();
        let settings = TelegramSettings {
            allowed_user_ids: vec![42],
            ..TelegramSettings::default()
        };

        handle_update(&inner, &client, &settings, message_update(1, 7, "/status"))
            .await
            .unwrap();
        assert!(client.messages.lock().await.is_empty());
        handle_update(&inner, &client, &settings, message_update(2, 42, "/status"))
            .await
            .unwrap();
        assert_eq!(client.messages.lock().await[0].chat_id, 42);
        assert!(
            client.messages.lock().await[0]
                .text
                .contains("Runtime: ready")
        );
    }

    #[tokio::test]
    async fn url_and_single_use_callback_create_one_owned_download() {
        let (inner, downloads) = test_inner().await;
        let client = FakeTelegramApi::default();
        let settings = TelegramSettings {
            allowed_user_ids: vec![42],
            notify_queued: true,
            ..TelegramSettings::default()
        };
        handle_update(
            &inner,
            &client,
            &settings,
            message_update(1, 42, "https://example.test/media"),
        )
        .await
        .unwrap();
        let callback_data = client.messages.lock().await[1]
            .keyboard
            .as_ref()
            .unwrap()
            .inline_keyboard[0][0]
            .callback_data
            .clone();

        handle_update(
            &inner,
            &client,
            &settings,
            callback_update(2, 42, callback_data.clone()),
        )
        .await
        .unwrap();
        let jobs = downloads.list().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(
            inner
                .storage
                .telegram_job_owner(jobs[0].id)
                .await
                .unwrap()
                .unwrap()
                .user_id,
            42
        );

        handle_update(
            &inner,
            &client,
            &settings,
            callback_update(3, 42, callback_data),
        )
        .await
        .unwrap();
        assert_eq!(downloads.list().await.unwrap().len(), 1);
        assert!(
            client
                .messages
                .lock()
                .await
                .last()
                .unwrap()
                .text
                .contains("already used")
        );
    }

    #[tokio::test]
    async fn terminal_notifications_are_owned_preference_aware_and_redacted() {
        let (inner, downloads) = test_inner().await;
        let client = FakeTelegramApi::default();
        inner
            .storage
            .save_telegram_settings(&TelegramSettings {
                enabled: true,
                allowed_user_ids: vec![42],
                notify_completed: true,
                privacy_acknowledged: true,
                ..TelegramSettings::default()
            })
            .await
            .unwrap();
        let mut job = downloads
            .create(DownloadRequest {
                url: "https://secret.example.test/watch?id=private".into(),
                title: Some("Safe title".into()),
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
                output_directory: Some(PathBuf::from("/private/download/path")),
            })
            .await
            .unwrap();
        inner
            .storage
            .associate_telegram_job(&TelegramJobOwner {
                job_id: job.id,
                user_id: 42,
                chat_id: 42,
            })
            .await
            .unwrap();
        job.status = DownloadStatus::Completed;

        deliver_notification(&inner, &client, &ApplicationEvent::DownloadCompleted(job))
            .await
            .unwrap();
        let messages = client.messages.lock().await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Completed: Safe title");
        assert!(!messages[0].text.contains("secret.example"));
        assert!(!messages[0].text.contains("/private"));
    }

    #[tokio::test]
    async fn managed_tasks_cancel_and_join_before_replacement() {
        let cancellation = CancellationToken::new();
        let observed = cancellation.clone();
        let task_cancellation = cancellation.clone();
        let handle = tokio::spawn(async move { task_cancellation.cancelled().await });
        let slot = tokio::sync::Mutex::new(Some(RunningTask {
            cancellation,
            handle,
        }));

        stop_task(&slot).await;
        assert!(observed.is_cancelled());
        assert!(slot.lock().await.is_none());
    }

    #[tokio::test]
    async fn playlists_require_a_second_confirmation_before_creating_jobs() {
        let (inner, downloads) = test_inner().await;
        let client = FakeTelegramApi::default();
        let action = TelegramPendingAction {
            id: Uuid::new_v4(),
            user_id: 42,
            chat_id: 42,
            source_url: "https://example.test/playlist".into(),
            media: playlist_media(),
            stage: TelegramPendingStage::ChooseMode,
            mode: None,
            expires_at: Utc::now() + chrono::Duration::minutes(1),
            consumed_at: None,
        };
        inner
            .storage
            .save_telegram_pending_action(&action)
            .await
            .unwrap();
        let identity = TelegramIdentity {
            user_id: 42,
            chat_id: 42,
            private_chat: true,
        };

        handle_callback(&inner, &client, identity, CallbackAction::Video(action.id))
            .await
            .unwrap();
        assert!(downloads.list().await.unwrap().is_empty());
        let confirmation = client.messages.lock().await[0]
            .keyboard
            .as_ref()
            .unwrap()
            .inline_keyboard[0][0]
            .callback_data
            .clone();
        handle_callback(
            &inner,
            &client,
            identity,
            CallbackAction::parse(&confirmation).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(downloads.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn stop_actions_apply_only_to_the_owning_telegram_user() {
        let (inner, downloads) = test_inner().await;
        let client = FakeTelegramApi::default();
        let job = downloads
            .create(DownloadRequest {
                url: "https://example.test/media".into(),
                title: Some("Owned".into()),
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
            })
            .await
            .unwrap();
        inner
            .storage
            .associate_telegram_job(&TelegramJobOwner {
                job_id: job.id,
                user_id: 42,
                chat_id: 42,
            })
            .await
            .unwrap();

        handle_callback(
            &inner,
            &client,
            TelegramIdentity {
                user_id: 7,
                chat_id: 7,
                private_chat: true,
            },
            CallbackAction::Stop(job.id),
        )
        .await
        .unwrap();
        assert!(downloads.stopped.lock().await.is_empty());
        handle_callback(
            &inner,
            &client,
            TelegramIdentity {
                user_id: 42,
                chat_id: 42,
                private_chat: true,
            },
            CallbackAction::Stop(job.id),
        )
        .await
        .unwrap();
        assert_eq!(*downloads.stopped.lock().await, vec![job.id]);
    }

    #[test]
    fn playlist_requests_preserve_order_and_require_downloadable_urls() {
        let action = TelegramPendingAction {
            id: Uuid::new_v4(),
            user_id: 42,
            chat_id: 42,
            source_url: "https://example.test/playlist".into(),
            media: MediaInfo {
                kind: MediaKind::Playlist,
                id: Some("playlist-id".into()),
                extractor: None,
                title: "Playlist".into(),
                webpage_url: None,
                duration_seconds: None,
                thumbnail_url: None,
                playlist_count: Some(2),
                entries: vec![
                    PlaylistEntry {
                        id: Some("one".into()),
                        title: "One".into(),
                        url: Some("https://example.test/one".into()),
                        duration_seconds: None,
                        thumbnail_url: None,
                    },
                    PlaylistEntry {
                        id: Some("missing".into()),
                        title: "Missing".into(),
                        url: None,
                        duration_seconds: None,
                        thumbnail_url: None,
                    },
                    PlaylistEntry {
                        id: Some("two".into()),
                        title: "Two".into(),
                        url: Some("https://example.test/two".into()),
                        duration_seconds: None,
                        thumbnail_url: None,
                    },
                ],
                formats: Vec::new(),
            },
            stage: TelegramPendingStage::ConfirmPlaylist,
            mode: Some(DownloadMode::Video),
            expires_at: Utc::now() + chrono::Duration::minutes(1),
            consumed_at: None,
        };
        let requests = requests_for_media(&action, DownloadMode::Video).unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].playlist.as_ref().unwrap().index, 1);
        assert_eq!(requests[1].playlist.as_ref().unwrap().index, 2);
    }

    fn playlist_media() -> MediaInfo {
        MediaInfo {
            kind: MediaKind::Playlist,
            id: Some("playlist-id".into()),
            extractor: None,
            title: "Playlist".into(),
            webpage_url: None,
            duration_seconds: None,
            thumbnail_url: None,
            playlist_count: Some(2),
            entries: vec![
                PlaylistEntry {
                    id: Some("one".into()),
                    title: "One".into(),
                    url: Some("https://example.test/one".into()),
                    duration_seconds: None,
                    thumbnail_url: None,
                },
                PlaylistEntry {
                    id: Some("two".into()),
                    title: "Two".into(),
                    url: Some("https://example.test/two".into()),
                    duration_seconds: None,
                    thumbnail_url: None,
                },
            ],
            formats: Vec::new(),
        }
    }
}
