//! Typed Telegram Bot API transport for Fetch.

use std::{fmt, sync::Arc, time::Duration};

use fetch_core::TelegramToken;
use rand::Rng;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

const DEFAULT_API_ROOT: &str = "https://api.telegram.org";
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[async_trait::async_trait]
pub trait TelegramApi: Send + Sync {
    async fn get_me(&self) -> Result<BotUser, TelegramError>;
    async fn get_updates(
        &self,
        offset: i64,
        long_poll_timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<Vec<Update>, TelegramError>;
    async fn send_message(&self, request: SendMessage<'_>) -> Result<Message, TelegramError>;
    async fn answer_callback_query(&self, callback_query_id: &str) -> Result<(), TelegramError>;
}

#[derive(Clone)]
pub struct BotApiClient {
    client: reqwest::Client,
    api_root: Arc<str>,
    token: TelegramToken,
    request_timeout: Duration,
}

impl BotApiClient {
    pub fn new(token: TelegramToken) -> Result<Self, TelegramError> {
        Self::with_api_root(token, DEFAULT_API_ROOT)
    }

    pub fn with_api_root(
        token: TelegramToken,
        api_root: impl Into<Arc<str>>,
    ) -> Result<Self, TelegramError> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .user_agent(concat!("Fetch/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| TelegramError::ClientConfiguration)?;
        Ok(Self {
            client,
            api_root: api_root.into(),
            token,
            request_timeout: Duration::from_secs(15),
        })
    }

    fn endpoint(&self, method: &str) -> String {
        format!(
            "{}/bot{}/{}",
            self.api_root.trim_end_matches('/'),
            self.token.expose(),
            method
        )
    }

    async fn post<T, B>(
        &self,
        method: &str,
        body: &B,
        timeout: Duration,
    ) -> Result<T, TelegramError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let response = self
            .client
            .post(self.endpoint(method))
            .timeout(timeout)
            .json(body)
            .send()
            .await
            .map_err(classify_request_error)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| TelegramError::Network)?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(TelegramError::ResponseTooLarge);
        }
        let envelope: ApiResponse<T> =
            serde_json::from_slice(&bytes).map_err(|_| TelegramError::MalformedResponse)?;
        if envelope.ok {
            return envelope.result.ok_or(TelegramError::MalformedResponse);
        }
        Err(classify_api_error(status, envelope))
    }
}

impl fmt::Debug for BotApiClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BotApiClient")
            .field("api_root", &self.api_root)
            .field("token", &"<redacted>")
            .field("request_timeout", &self.request_timeout)
            .finish()
    }
}

#[async_trait::async_trait]
impl TelegramApi for BotApiClient {
    async fn get_me(&self) -> Result<BotUser, TelegramError> {
        self.post("getMe", &EmptyRequest {}, self.request_timeout)
            .await
    }

    async fn get_updates(
        &self,
        offset: i64,
        long_poll_timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<Vec<Update>, TelegramError> {
        let timeout_seconds = long_poll_timeout.as_secs().clamp(1, 30);
        let request = GetUpdatesRequest {
            offset,
            timeout: timeout_seconds,
            allowed_updates: ["message", "callback_query"],
        };
        let request_timeout = long_poll_timeout + Duration::from_secs(10);
        tokio::select! {
            _ = cancellation.cancelled() => Err(TelegramError::Cancelled),
            result = self.post("getUpdates", &request, request_timeout) => result,
        }
    }

    async fn send_message(&self, request: SendMessage<'_>) -> Result<Message, TelegramError> {
        self.post("sendMessage", &request, self.request_timeout)
            .await
    }

    async fn answer_callback_query(&self, callback_query_id: &str) -> Result<(), TelegramError> {
        let _: bool = self
            .post(
                "answerCallbackQuery",
                &AnswerCallbackRequest { callback_query_id },
                self.request_timeout,
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TelegramError {
    #[error("Telegram client could not be configured")]
    ClientConfiguration,
    #[error("Telegram request was cancelled")]
    Cancelled,
    #[error("Telegram request timed out")]
    Timeout,
    #[error("Telegram is unreachable")]
    Network,
    #[error("Telegram rejected the bot token")]
    Unauthorized,
    #[error("Telegram denied this bot operation")]
    Forbidden,
    #[error("another Telegram getUpdates poller is using this bot token")]
    PollingConflict,
    #[error("Telegram rate limited Fetch; retry after {retry_after_seconds} seconds")]
    RateLimited { retry_after_seconds: u64 },
    #[error("Telegram returned malformed data")]
    MalformedResponse,
    #[error("Telegram response exceeded the safety limit")]
    ResponseTooLarge,
    #[error("Telegram API error {code}")]
    Api { code: i64 },
}

impl TelegramError {
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout
                | Self::Network
                | Self::RateLimited { .. }
                | Self::Api { code: 500..=599 }
        )
    }
}

#[derive(Debug, Clone)]
pub struct Backoff {
    attempt: u32,
    initial: Duration,
    maximum: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            attempt: 0,
            initial: Duration::from_secs(1),
            maximum: Duration::from_secs(30),
        }
    }
}

impl Backoff {
    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn next_delay(&mut self, error: &TelegramError) -> Duration {
        if let TelegramError::RateLimited {
            retry_after_seconds,
        } = error
        {
            // Telegram's explicit retry-after is authoritative. Keep only a
            // defensive upper bound so a malformed response cannot park the
            // manager indefinitely; cancellation remains immediate.
            return Duration::from_secs(*retry_after_seconds).min(Duration::from_secs(3_600));
        }
        let exponent = self.attempt.min(5);
        self.attempt = self.attempt.saturating_add(1);
        let base = self.initial.saturating_mul(1_u32 << exponent);
        let capped = base.min(self.maximum);
        let jitter_limit = capped.as_millis().saturating_div(4) as u64;
        let jitter = rand::rng().random_range(0..=jitter_limit);
        capped
            .saturating_add(Duration::from_millis(jitter))
            .min(self.maximum)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BotUser {
    pub id: i64,
    pub is_bot: bool,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub from: Option<User>,
    pub chat: Chat,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SendMessage<'a> {
    pub chat_id: i64,
    pub text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<&'a InlineKeyboardMarkup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    pub callback_data: String,
}

#[derive(Serialize)]
struct EmptyRequest {}

#[derive(Serialize)]
struct GetUpdatesRequest<'a> {
    offset: i64,
    timeout: u64,
    allowed_updates: [&'a str; 2],
}

#[derive(Serialize)]
struct AnswerCallbackRequest<'a> {
    callback_query_id: &'a str,
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    ok: bool,
    result: Option<T>,
    error_code: Option<i64>,
    parameters: Option<ResponseParameters>,
}

#[derive(Deserialize)]
struct ResponseParameters {
    retry_after: Option<u64>,
}

fn classify_request_error(error: reqwest::Error) -> TelegramError {
    if error.is_timeout() {
        TelegramError::Timeout
    } else {
        TelegramError::Network
    }
}

fn classify_api_error<T>(status: StatusCode, response: ApiResponse<T>) -> TelegramError {
    let code = response.error_code.unwrap_or(i64::from(status.as_u16()));
    if let Some(retry_after_seconds) = response
        .parameters
        .and_then(|parameters| parameters.retry_after)
    {
        return TelegramError::RateLimited {
            retry_after_seconds,
        };
    }
    match code {
        401 => TelegramError::Unauthorized,
        403 => TelegramError::Forbidden,
        409 => TelegramError::PollingConflict,
        _ => TelegramError::Api { code },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncomingMessage {
    Start,
    Help,
    Status,
    Downloads,
    SourceUrl(String),
    Unsupported,
}

impl IncomingMessage {
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        if let Some(command) = text.strip_prefix('/') {
            let command = command
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .split('@')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            return match command.as_str() {
                "start" => Self::Start,
                "help" => Self::Help,
                "status" => Self::Status,
                "downloads" => Self::Downloads,
                _ => Self::Unsupported,
            };
        }
        match url::Url::parse(text) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => Self::SourceUrl(text.to_owned()),
            _ => Self::Unsupported,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackAction {
    Video(uuid::Uuid),
    Audio(uuid::Uuid),
    ConfirmPlaylist(uuid::Uuid),
    Cancel(uuid::Uuid),
    Stop(uuid::Uuid),
}

impl CallbackAction {
    pub fn encode(self) -> String {
        let (verb, id) = match self {
            Self::Video(id) => ("v", id),
            Self::Audio(id) => ("a", id),
            Self::ConfirmPlaylist(id) => ("p", id),
            Self::Cancel(id) => ("c", id),
            Self::Stop(id) => ("s", id),
        };
        format!("{verb}:{id}")
    }

    pub fn parse(value: &str) -> Option<Self> {
        let (verb, id) = value.split_once(':')?;
        let id = id.parse().ok()?;
        match verb {
            "v" => Some(Self::Video(id)),
            "a" => Some(Self::Audio(id)),
            "p" => Some(Self::ConfirmPlaylist(id)),
            "c" => Some(Self::Cancel(id)),
            "s" => Some(Self::Stop(id)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Json, Router, extract::State, routing::post};
    use serde_json::{Value, json};
    use tokio::sync::Mutex;

    use super::*;

    #[tokio::test]
    async fn parses_updates_and_sends_typed_requests_without_exposing_token_in_debug() {
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route("/botfixture/getMe", post(|| async { Json(json!({"ok": true, "result": {"id": 1, "is_bot": true, "username": "fetch_bot"}})) }))
            .route("/botfixture/getUpdates", post(record_updates))
            .with_state(bodies.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let token = TelegramToken::try_from("fixture".to_owned()).unwrap();
        let client = BotApiClient::with_api_root(token, format!("http://{address}")).unwrap();

        assert!(!format!("{client:?}").contains("fixture"));
        assert_eq!(
            client.get_me().await.unwrap().username.as_deref(),
            Some("fetch_bot")
        );
        let updates = client
            .get_updates(12, Duration::from_secs(1), &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(updates[0].update_id, 12);
        let body = bodies.lock().await.pop().unwrap();
        assert_eq!(body["offset"], 12);
        assert_eq!(
            body["allowed_updates"],
            json!(["message", "callback_query"])
        );
    }

    async fn record_updates(
        State(bodies): State<Arc<Mutex<Vec<Value>>>>,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        bodies.lock().await.push(body);
        Json(json!({
            "ok": true,
            "result": [{
                "update_id": 12,
                "message": {
                    "message_id": 2,
                    "from": {"id": 42, "is_bot": false},
                    "chat": {"id": 42, "type": "private"},
                    "text": "/status"
                }
            }]
        }))
    }

    #[tokio::test]
    async fn classifies_retry_after_and_cancellation() {
        let app = Router::new().route(
            "/botfixture/getMe",
            post(|| async {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "ok": false,
                        "error_code": 429,
                        "description": "slow down",
                        "parameters": {"retry_after": 7}
                    })),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = BotApiClient::with_api_root(
            TelegramToken::try_from("fixture".to_owned()).unwrap(),
            format!("http://{address}"),
        )
        .unwrap();
        assert_eq!(
            client.get_me().await.unwrap_err(),
            TelegramError::RateLimited {
                retry_after_seconds: 7
            }
        );

        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert_eq!(
            client
                .get_updates(0, Duration::from_secs(1), &cancelled)
                .await
                .unwrap_err(),
            TelegramError::Cancelled
        );
    }

    #[test]
    fn parses_only_supported_commands_and_http_source_urls() {
        assert_eq!(
            IncomingMessage::parse(" /STATUS@FetchBot "),
            IncomingMessage::Status
        );
        assert_eq!(
            IncomingMessage::parse("https://example.test/watch?v=1"),
            IncomingMessage::SourceUrl("https://example.test/watch?v=1".into())
        );
        assert_eq!(
            IncomingMessage::parse("file:///tmp/video"),
            IncomingMessage::Unsupported
        );
        assert_eq!(
            IncomingMessage::parse("/unknown"),
            IncomingMessage::Unsupported
        );
    }

    #[test]
    fn callback_payloads_are_opaque_bounded_and_round_trip() {
        let id = uuid::Uuid::new_v4();
        for action in [
            CallbackAction::Video(id),
            CallbackAction::Audio(id),
            CallbackAction::ConfirmPlaylist(id),
            CallbackAction::Cancel(id),
            CallbackAction::Stop(id),
        ] {
            let encoded = action.encode();
            assert!(encoded.len() <= 64);
            assert_eq!(CallbackAction::parse(&encoded), Some(action));
        }
        assert_eq!(CallbackAction::parse("v:not-a-uuid"), None);
    }

    #[test]
    fn backoff_is_capped_and_honors_retry_after() {
        let mut backoff = Backoff::default();
        for _ in 0..20 {
            assert!(backoff.next_delay(&TelegramError::Network) <= Duration::from_secs(30));
        }
        assert_eq!(
            backoff.next_delay(&TelegramError::RateLimited {
                retry_after_seconds: 7,
            }),
            Duration::from_secs(7)
        );
        assert_eq!(
            backoff.next_delay(&TelegramError::RateLimited {
                retry_after_seconds: 120,
            }),
            Duration::from_secs(120)
        );
    }

    #[tokio::test]
    #[ignore = "requires explicit FETCH_TELEGRAM_BOT_TOKEN and Telegram network access"]
    async fn live_bot_identity_smoke_is_opt_in() {
        let value = std::env::var("FETCH_TELEGRAM_BOT_TOKEN")
            .expect("set FETCH_TELEGRAM_BOT_TOKEN to run the ignored live smoke");
        let token = TelegramToken::try_from(value).expect("the live smoke token is invalid");
        let bot = BotApiClient::new(token)
            .expect("could not build Telegram client")
            .get_me()
            .await
            .expect("Telegram getMe smoke failed");
        assert!(bot.is_bot);
    }
}
