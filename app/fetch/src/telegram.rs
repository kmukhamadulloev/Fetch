use std::{
    collections::{HashMap, VecDeque},
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::Utc;
use fetch_core::{
    AppStatus, ApplicationEvent, DownloadMode, DownloadOperations, DownloadRequest, DownloadStatus,
    EventBus, FetchError, MediaAnalysis, MediaKind, PlaylistContext, TelegramConnectionState,
    TelegramIdentity, TelegramIntegration, TelegramJobOwner, TelegramOperations,
    TelegramPendingAction, TelegramPendingStage, TelegramSecretStore, TelegramSettings,
    TelegramStatus, TelegramToken, TelegramTokenSource,
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
    ) -> Self {
        Self {
            inner: Arc::new(ManagerInner {
                storage,
                secrets: NativeTelegramSecretStore,
                media,
                downloads,
                events,
                app_status,
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
            *self.inner.status.write().await = TelegramStatus {
                token_configured: current.token_configured,
                token_source: current.token_source,
                ..TelegramStatus::default()
            };
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
            *self.inner.status.write().await = TelegramStatus {
                state: TelegramConnectionState::Error,
                token_configured: false,
                token_source: TelegramTokenSource::Missing,
                error: Some("Add a Telegram bot token before enabling the integration".into()),
                ..TelegramStatus::default()
            };
            return;
        };
        let client = match BotApiClient::new(token) {
            Ok(client) => client,
            Err(error) => {
                self.set_telegram_error(&error, source).await;
                return;
            }
        };
        *self.inner.status.write().await = TelegramStatus {
            state: TelegramConnectionState::Connecting,
            token_configured: true,
            token_source: source,
            ..TelegramStatus::default()
        };
        let cancellation = CancellationToken::new();
        let task_cancellation = cancellation.clone();
        let inner = self.inner.clone();
        let handle = tokio::spawn(async move {
            run_poller(inner, client, settings, source, task_cancellation).await;
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
        let mut status = self.inner.status.write().await;
        status.state = TelegramConnectionState::Error;
        status.error = Some(public.to_owned());
    }

    async fn set_telegram_error(&self, error: &TelegramError, source: TelegramTokenSource) {
        let public = error.to_string();
        self.set_error(&public, &public).await;
        let mut status = self.inner.status.write().await;
        status.token_configured = true;
        status.token_source = source;
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
        {
            let mut status = self.inner.status.write().await;
            status.token_configured = true;
            status.token_source = if token_from_environment()?.is_some() {
                TelegramTokenSource::Environment
            } else {
                TelegramTokenSource::Native
            };
        }
        self.restart_poller().await;
        self.get_integration().await
    }

    async fn delete_token(&self) -> Result<TelegramIntegration, FetchError> {
        self.inner.secrets.delete().await?;
        {
            let mut status = self.inner.status.write().await;
            status.token_configured = token_from_environment()?.is_some();
            status.token_source = if status.token_configured {
                TelegramTokenSource::Environment
            } else {
                TelegramTokenSource::Missing
            };
        }
        self.restart_poller().await;
        self.get_integration().await
    }

    async fn test_connection(&self) -> Result<TelegramIntegration, FetchError> {
        let (token, source) = self.effective_token().await?;
        let token = token.ok_or_else(|| {
            FetchError::InvalidSettings("Add a Telegram bot token before testing".into())
        })?;
        let client =
            BotApiClient::new(token).map_err(|error| FetchError::Internal(error.to_string()))?;
        match client.get_me().await {
            Ok(bot) => {
                let mut status = self.inner.status.write().await;
                status.token_configured = true;
                status.token_source = source;
                status.bot_username = bot.username;
                status.last_success_at = Some(Utc::now());
                status.error = None;
                if status.state == TelegramConnectionState::Error {
                    status.state = TelegramConnectionState::Disabled;
                }
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

async fn run_poller(
    inner: Arc<ManagerInner>,
    client: BotApiClient,
    settings: TelegramSettings,
    source: TelegramTokenSource,
    cancellation: CancellationToken,
) {
    match client.get_me().await {
        Ok(bot) => {
            *inner.status.write().await = TelegramStatus {
                state: TelegramConnectionState::Connected,
                token_configured: true,
                token_source: source,
                bot_username: bot.username,
                last_success_at: Some(Utc::now()),
                error: None,
            };
            let _ = inner
                .storage
                .append_log("info", "telegram", "Telegram bot connected", None)
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
        match client
            .get_updates(offset, Duration::from_secs(25), &cancellation)
            .await
        {
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
                                    .release_telegram_update_claim(next_offset.saturating_sub(1))
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
                let mut status = inner.status.write().await;
                status.state = TelegramConnectionState::Connected;
                status.last_success_at = Some(Utc::now());
                status.error = None;
            }
            Err(TelegramError::Cancelled) => return,
            Err(error) if error.retryable() => {
                let delay = backoff.next_delay(&error);
                {
                    let mut status = inner.status.write().await;
                    status.state = TelegramConnectionState::BackingOff;
                    status.error = Some(error.to_string());
                }
                tokio::select! {
                    _ = cancellation.cancelled() => return,
                    _ = tokio::time::sleep(delay) => {}
                }
            }
            Err(error) => {
                record_poller_error(&inner, &error, source).await;
                return;
            }
        }
    }
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
    *inner.status.write().await = TelegramStatus {
        state: TelegramConnectionState::Error,
        token_configured: true,
        token_source: source,
        error: Some(message),
        ..TelegramStatus::default()
    };
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
        let (job, completed) = match &event {
            ApplicationEvent::DownloadCompleted(job) => (job, true),
            ApplicationEvent::DownloadFailed(job) | ApplicationEvent::DownloadStopped(job) => {
                (job, false)
            }
            _ => continue,
        };
        let Ok(Some(owner)) = inner.storage.telegram_job_owner(job.id).await else {
            continue;
        };
        let Ok(settings) = inner.storage.load_telegram_settings().await else {
            continue;
        };
        let notifications_enabled = if completed {
            settings.notify_completed
        } else {
            settings.notify_failed
        };
        if !settings.enabled || !notifications_enabled {
            continue;
        }
        let Ok((Some(token), _)) = effective_token_for(&inner.secrets).await else {
            continue;
        };
        let Ok(client) = BotApiClient::new(token) else {
            continue;
        };
        let state = match job.status {
            DownloadStatus::Completed => "Completed",
            DownloadStatus::Failed => "Failed",
            DownloadStatus::Stopped => "Stopped",
            _ => continue,
        };
        let text = format!(
            "{state}: {}",
            job.request.title.as_deref().unwrap_or("Untitled download")
        );
        if let Err(error) = send_text(&client, owner.chat_id, &text, None).await {
            tracing::warn!(subsystem = "telegram", error = %error.public_message(), "could not send Telegram terminal notification");
        }
    }
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
    struct FakeDownloads;

    #[derive(Default)]
    struct FakeTelegramApi {
        messages: Mutex<Vec<(i64, String)>>,
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
            Ok(DownloadJob::new(request))
        }
        async fn list(&self) -> Result<Vec<DownloadJob>, FetchError> {
            Ok(Vec::new())
        }
        async fn get(&self, _id: Uuid) -> Result<DownloadJob, FetchError> {
            Err(FetchError::NotFound)
        }
        async fn stop(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
            self.get(id).await
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
            self.messages
                .lock()
                .await
                .push((request.chat_id, request.text.into()));
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

    async fn test_inner() -> ManagerInner {
        ManagerInner {
            storage: Arc::new(
                Storage::open(std::path::Path::new(":memory:"))
                    .await
                    .unwrap(),
            ),
            secrets: NativeTelegramSecretStore,
            media: Arc::new(FakeMedia),
            downloads: Arc::new(FakeDownloads),
            events: EventBus::default(),
            app_status: fetch_core::StatusService::new("0.1.4", true, true),
            status: RwLock::new(TelegramStatus::default()),
            poller: tokio::sync::Mutex::new(None),
            notifier: tokio::sync::Mutex::new(None),
            command_windows: tokio::sync::Mutex::new(HashMap::new()),
        }
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
    async fn ignores_unauthorized_updates_and_answers_authorized_status() {
        let inner = test_inner().await;
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
        assert_eq!(client.messages.lock().await[0].0, 42);
        assert!(client.messages.lock().await[0].1.contains("Runtime: ready"));
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
}
