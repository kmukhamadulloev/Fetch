//! Axum HTTP transport, SSE, and embedded production frontend.

use std::{
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    sync::{Arc, RwLock},
    time::Duration,
};

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response, Sse, sse::Event},
    routing::{delete, get, post, put},
};
use fetch_core::{
    ApplicationEvent, ApplicationSettings, CompletedOperations, DiagnosticOperations,
    DownloadOperations, DownloadRequest, DownloadStatus, ErrorCode, ErrorResponse, EventBus,
    FetchError, ListenerOperations, MediaAnalysis, PlaybackProgressUpdate, ProxyOperations,
    ProxySettings, RuntimeStatus, SettingsOperations, StatusService, TelegramIntegration,
    TelegramOperations, TelegramSettings, TelegramToken,
};
use fetch_runtime::RuntimeManager;
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::{io::ReaderStream, sync::CancellationToken};

#[derive(Clone)]
pub struct ServerServices {
    pub status: StatusService,
    pub runtime: RuntimeManager,
    pub media: Arc<dyn MediaAnalysis>,
    pub downloads: Arc<dyn DownloadOperations>,
    pub events: EventBus,
    pub completed: Arc<dyn CompletedOperations>,
    pub settings: Arc<dyn SettingsOperations>,
    pub proxy: Arc<dyn ProxyOperations>,
    pub listener: Arc<dyn ListenerOperations>,
    pub network_policy: NetworkPolicy,
    pub diagnostics: Arc<dyn DiagnosticOperations>,
    pub telegram: Arc<dyn TelegramOperations>,
    pub shutdown: CancellationToken,
}

#[derive(Clone)]
pub struct NetworkPolicy {
    networks: Arc<RwLock<Vec<ipnet::IpNet>>>,
}

impl NetworkPolicy {
    pub fn new(networks: &[String]) -> Result<Self, FetchError> {
        Ok(Self {
            networks: Arc::new(RwLock::new(parse_networks(networks)?)),
        })
    }

    pub fn allows(&self, address: IpAddr) -> bool {
        address.is_loopback()
            || self
                .networks
                .read()
                .expect("network policy lock is not poisoned")
                .iter()
                .any(|network| network.contains(&address))
    }

    fn update(&self, networks: &[String]) -> Result<(), FetchError> {
        *self
            .networks
            .write()
            .expect("network policy lock is not poisoned") = parse_networks(networks)?;
        Ok(())
    }
}

fn parse_networks(networks: &[String]) -> Result<Vec<ipnet::IpNet>, FetchError> {
    networks
        .iter()
        .map(|network| {
            network.parse().map_err(|_| {
                FetchError::InvalidSettings(format!("{network} is not a valid CIDR network"))
            })
        })
        .collect()
}

fn is_host_client(address: IpAddr) -> bool {
    address.is_loopback()
        || if_addrs::get_if_addrs().is_ok_and(|interfaces| {
            interfaces
                .into_iter()
                .any(|interface| interface.ip() == address)
        })
}

#[derive(Clone)]
struct AppState {
    services: ServerServices,
}

#[derive(Embed)]
#[folder = "../../web/dist"]
struct WebAssets;

pub fn router(services: ServerServices) -> Router {
    let policy = services.network_policy.clone();
    let state = AppState { services };
    Router::new()
        .route("/api/status", get(api_status))
        .route("/api/media/analyze", post(analyze_media))
        .route("/api/downloads", get(list_downloads).post(create_download))
        .route(
            "/api/downloads/{id}",
            get(get_download).delete(delete_download),
        )
        .route("/api/downloads/{id}/stop", post(stop_download))
        .route("/api/downloads/{id}/resume", post(resume_download))
        .route("/api/downloads/{id}/retry", post(retry_download))
        .route("/api/completed", get(list_completed))
        .route("/api/history", get(history))
        .route("/api/files/{id}", delete(delete_completed_file))
        .route("/api/files/{id}/download", get(download_file))
        .route("/api/files/{id}/stream", get(stream_file))
        .route("/api/files/{id}/thumbnail", get(thumbnail_file))
        .route(
            "/api/files/{id}/metadata",
            get(get_metadata)
                .put(put_metadata)
                .layer(axum::extract::DefaultBodyLimit::max(12 * 1024 * 1024)),
        )
        .route("/api/files/{id}/processing", get(processing_capabilities))
        .route("/api/files/{id}/processes", post(start_export))
        .route("/api/processes", get(list_processes))
        .route("/api/processes/{id}/cancel", post(cancel_process))
        .route("/api/processes/{id}/retry", post(retry_process))
        .route("/api/files/{id}/metadata/status", get(metadata_status))
        .route("/api/files/{id}/metadata/artwork", get(metadata_artwork))
        .route("/api/files/{id}/reveal", post(reveal_completed_file))
        .route(
            "/api/files/{id}/progress",
            put(save_playback_progress).delete(clear_playback_progress),
        )
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/proxy", get(get_proxy).put(put_proxy))
        .route("/api/telegram", get(get_telegram))
        .route("/api/telegram/settings", put(put_telegram_settings))
        .route(
            "/api/telegram/token",
            put(put_telegram_token).delete(delete_telegram_token),
        )
        .route("/api/telegram/test", post(test_telegram))
        .route("/api/telegram/restart", post(restart_telegram))
        .route("/api/network", get(network_info))
        .route("/api/logs", get(logs).delete(clear_logs))
        .route("/api/diagnostics", get(diagnostics))
        .route("/api/runtime", get(runtime_status))
        .route("/api/runtime/javascript", get(javascript_runtime_status))
        .route("/api/runtime/{component}/install", post(runtime_install))
        .route("/api/runtime/{component}/update", post(runtime_update))
        .route("/api/runtime/{component}/repair", post(runtime_repair))
        .route("/api/events", get(events))
        .nest("/api", Router::new().fallback(api_not_found))
        .fallback(spa)
        .with_state(state)
        .layer(from_fn_with_state(policy, enforce_network))
}

async fn enforce_network(
    State(policy): State<NetworkPolicy>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    if request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|ConnectInfo(address)| !policy.allows(address.ip()))
    {
        return ApiError(FetchError::NetworkDenied).into_response();
    }
    next.run(request).await
}

async fn api_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.services.status.status())
}

#[derive(Debug, Deserialize)]
struct AnalyzeRequest {
    url: String,
}

async fn analyze_media(
    State(state): State<AppState>,
    Json(request): Json<AnalyzeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.media.analyze(&request.url).await?))
}

async fn runtime_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.services.runtime.components().await)
}

async fn javascript_runtime_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.services.runtime.javascript_runtimes().await)
}

async fn list_downloads(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.downloads.list().await?))
}

async fn create_download(
    State(state): State<AppState>,
    Json(request): Json<DownloadRequest>,
) -> Result<impl IntoResponse, ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(state.services.downloads.create(request).await?),
    ))
}

async fn get_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.downloads.get(parse_uuid(&id)?).await?))
}

async fn stop_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.downloads.stop(parse_uuid(&id)?).await?))
}

async fn resume_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state.services.downloads.resume(parse_uuid(&id)?).await?,
    ))
}

async fn retry_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state.services.downloads.retry(parse_uuid(&id)?).await?,
    ))
}

async fn delete_download(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.services.downloads.delete(parse_uuid(&id)?).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn parse_uuid(value: &str) -> Result<uuid::Uuid, ApiError> {
    value
        .parse()
        .map_err(|_| ApiError(FetchError::InvalidRequest("invalid identifier".into())))
}

async fn list_completed(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.completed.list_completed().await?))
}

async fn processing_capabilities(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<fetch_core::ProcessingCapabilities>, ApiError> {
    Ok(Json(
        state
            .services
            .completed
            .processing_capabilities(parse_uuid(&id)?)
            .await?,
    ))
}
async fn start_export(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<fetch_core::ExportRequest>,
) -> Result<(StatusCode, Json<fetch_core::ProcessingJob>), ApiError> {
    request.validate()?;
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .services
                .completed
                .start_export(parse_uuid(&id)?, request)
                .await?,
        ),
    ))
}
async fn list_processes(
    State(state): State<AppState>,
) -> Result<Json<Vec<fetch_core::ProcessingJob>>, ApiError> {
    Ok(Json(state.services.completed.processes().await?))
}
async fn cancel_process(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<fetch_core::ProcessingJob>, ApiError> {
    Ok(Json(
        state
            .services
            .completed
            .cancel_process(parse_uuid(&id)?)
            .await?,
    ))
}
async fn retry_process(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<fetch_core::ProcessingJob>), ApiError> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .services
                .completed
                .retry_process(parse_uuid(&id)?)
                .await?,
        ),
    ))
}

async fn get_metadata(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state.services.completed.metadata(parse_uuid(&id)?).await?,
    ))
}
async fn put_metadata(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<fetch_core::MetadataUpdate>,
) -> Result<impl IntoResponse, ApiError> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            state
                .services
                .completed
                .update_metadata(parse_uuid(&id)?, update)
                .await?,
        ),
    ))
}
async fn metadata_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state
            .services
            .completed
            .metadata_status(parse_uuid(&id)?)
            .await?,
    ))
}
async fn metadata_artwork(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let bytes = state
        .services
        .completed
        .metadata_artwork(parse_uuid(&id)?)
        .await?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    ))
}

async fn delete_completed_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .services
        .completed
        .delete_completed(parse_uuid(&id)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reveal_completed_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    request: axum::extract::Request,
) -> Result<impl IntoResponse, ApiError> {
    if !request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|ConnectInfo(address)| is_host_client(address.ip()))
    {
        return Err(ApiError(FetchError::LocalClientRequired));
    }
    state
        .services
        .completed
        .reveal_completed(parse_uuid(&id)?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn save_playback_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<PlaybackProgressUpdate>,
) -> Result<impl IntoResponse, ApiError> {
    let progress = state
        .services
        .completed
        .save_playback_progress(parse_uuid(&id)?, update)
        .await?;
    state
        .services
        .events
        .publish(ApplicationEvent::PlaybackProgressUpdated(progress.clone()));
    Ok(Json(progress))
}

async fn clear_playback_progress(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let id = parse_uuid(&id)?;
    state.services.completed.clear_playback_progress(id).await?;
    state
        .services
        .events
        .publish(ApplicationEvent::PlaybackProgressCleared(id));
    Ok(StatusCode::NO_CONTENT)
}

async fn history(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let jobs = state
        .services
        .downloads
        .list()
        .await?
        .into_iter()
        .filter(|job| {
            matches!(
                job.status,
                DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Stopped
            )
        })
        .collect::<Vec<_>>();
    Ok(Json(jobs))
}

async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    serve_completed_file(state, id, headers, true).await
}

async fn stream_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    serve_completed_file(state, id, headers, false).await
}

async fn thumbnail_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let completed = state
        .services
        .completed
        .get_completed(parse_uuid(&id)?)
        .await?;
    let path = completed.thumbnail_path.ok_or(FetchError::FileNotFound)?;
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|_| ApiError(FetchError::FileNotFound))?;
    if !metadata.is_file() {
        return Err(ApiError(FetchError::FileNotFound));
    }
    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| ApiError(FetchError::FileNotFound))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            mime_guess::from_path(&path)
                .first_or_octet_stream()
                .as_ref(),
        )
        .header(header::CONTENT_LENGTH, metadata.len())
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from_stream(ReaderStream::new(file)))
        .expect("thumbnail response headers are valid"))
}

async fn serve_completed_file(
    state: AppState,
    id: String,
    headers: HeaderMap,
    attachment: bool,
) -> Result<Response, ApiError> {
    let completed = state
        .services
        .completed
        .get_completed(parse_uuid(&id)?)
        .await?;
    let metadata = tokio::fs::metadata(&completed.path)
        .await
        .map_err(|_| ApiError(FetchError::FileNotFound))?;
    if !metadata.is_file() {
        return Err(ApiError(FetchError::FileNotFound));
    }
    let length = metadata.len();
    let range = match headers.get(header::RANGE) {
        Some(value) => match value
            .to_str()
            .ok()
            .and_then(|value| parse_range(value, length))
        {
            Some(range) => Some(range),
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{length}"))
                    .body(Body::empty())
                    .expect("range error response is valid"));
            }
        },
        None => None,
    };
    let (start, end, status) = range
        .map(|(start, end)| (start, end, StatusCode::PARTIAL_CONTENT))
        .unwrap_or((0, length.saturating_sub(1), StatusCode::OK));
    let response_length = if length == 0 { 0 } else { end - start + 1 };
    let mut file = tokio::fs::File::open(&completed.path)
        .await
        .map_err(|_| ApiError(FetchError::FileNotFound))?;
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|error| ApiError(FetchError::Internal(error.to_string())))?;
    let stream = ReaderStream::new(file.take(response_length));
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, completed.mime_type)
        .header(header::CONTENT_LENGTH, response_length)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if let Some((start, end)) = range {
        builder = builder.header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{length}"),
        );
    }
    if attachment {
        let safe_filename = completed
            .filename
            .chars()
            .filter(|character| !character.is_control() && *character != '"' && *character != '\\')
            .collect::<String>();
        builder = builder.header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!("attachment; filename=\"{safe_filename}\""))
                .map_err(|error| ApiError(FetchError::Internal(error.to_string())))?,
        );
    }
    Ok(builder
        .body(Body::from_stream(stream))
        .expect("file response headers are valid"))
}

fn parse_range(value: &str, length: u64) -> Option<(u64, u64)> {
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') || length == 0 {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?.min(length);
        return (suffix > 0).then_some((length - suffix, length - 1));
    }
    let start = start.parse::<u64>().ok()?;
    if start >= length {
        return None;
    }
    let end = if end.is_empty() {
        length - 1
    } else {
        end.parse::<u64>().ok()?.min(length - 1)
    };
    (start <= end).then_some((start, end))
}

async fn get_settings(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.settings.get_settings().await?))
}

fn require_host_client(
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<(), ApiError> {
    match connection {
        Some(Extension(ConnectInfo(address))) if is_host_client(address.ip()) => Ok(()),
        _ => Err(ApiError(FetchError::LocalClientRequired)),
    }
}

async fn get_proxy(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<impl IntoResponse, ApiError> {
    require_host_client(connection)?;
    Ok(Json(state.services.proxy.get_proxy().await?))
}

async fn put_proxy(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
    Json(settings): Json<ProxySettings>,
) -> Result<impl IntoResponse, ApiError> {
    require_host_client(connection)?;
    Ok(Json(state.services.proxy.put_proxy(settings).await?))
}

async fn get_telegram(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<Json<TelegramIntegration>, ApiError> {
    require_host_client(connection)?;
    Ok(Json(state.services.telegram.get_integration().await?))
}

async fn put_telegram_settings(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
    Json(settings): Json<TelegramSettings>,
) -> Result<Json<TelegramIntegration>, ApiError> {
    require_host_client(connection)?;
    Ok(Json(state.services.telegram.put_settings(settings).await?))
}

#[derive(Deserialize)]
struct TelegramTokenRequest {
    token: TelegramToken,
}

async fn put_telegram_token(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
    Json(request): Json<TelegramTokenRequest>,
) -> Result<Json<TelegramIntegration>, ApiError> {
    require_host_client(connection)?;
    Ok(Json(
        state.services.telegram.put_token(request.token).await?,
    ))
}

async fn delete_telegram_token(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<Json<TelegramIntegration>, ApiError> {
    require_host_client(connection)?;
    Ok(Json(state.services.telegram.delete_token().await?))
}

async fn restart_telegram(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<Json<TelegramIntegration>, ApiError> {
    require_host_client(connection)?;
    Ok(Json(state.services.telegram.restart().await?))
}

async fn test_telegram(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<Json<TelegramIntegration>, ApiError> {
    require_host_client(connection)?;
    Ok(Json(state.services.telegram.test_connection().await?))
}

async fn put_settings(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
    Json(settings): Json<ApplicationSettings>,
) -> Result<impl IntoResponse, ApiError> {
    settings.validate_basic()?;
    let previous = state.services.settings.get_settings().await?;
    if settings.start_with_system != previous.start_with_system
        && connection.is_some_and(|Extension(ConnectInfo(address))| !is_host_client(address.ip()))
    {
        return Err(ApiError(FetchError::LocalClientRequired));
    }
    state
        .services
        .downloads
        .update_defaults(
            settings.download_directory.clone(),
            settings.concurrent_downloads,
            settings.ytdlp_js_runtime,
        )
        .await?;
    let saved = match state.services.settings.put_settings(settings).await {
        Ok(saved) => saved,
        Err(error) => {
            let _ = state
                .services
                .downloads
                .update_defaults(
                    previous.download_directory.clone(),
                    previous.concurrent_downloads,
                    previous.ytdlp_js_runtime,
                )
                .await;
            return Err(ApiError(error));
        }
    };
    let address = SocketAddr::new(saved.bind_address, saved.port);
    let listener_changed = match state.services.listener.rebind(address).await {
        Ok(changed) => changed,
        Err(error) => {
            let _ = state.services.settings.put_settings(previous.clone()).await;
            let _ = state
                .services
                .downloads
                .update_defaults(
                    previous.download_directory.clone(),
                    previous.concurrent_downloads,
                    previous.ytdlp_js_runtime,
                )
                .await;
            return Err(ApiError(error));
        }
    };
    state
        .services
        .network_policy
        .update(&saved.allowed_networks)?;
    Ok(Json(SettingsSaveResponse {
        settings: saved,
        listener_changed,
    }))
}

#[derive(Debug, Serialize)]
struct SettingsSaveResponse {
    #[serde(flatten)]
    settings: ApplicationSettings,
    listener_changed: bool,
}

#[derive(Debug, Serialize)]
struct NetworkInfo {
    bind_address: IpAddr,
    port: u16,
    urls: Vec<String>,
    authentication: bool,
    restart_required_after_bind_change: bool,
    local_client: bool,
}

async fn network_info(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<impl IntoResponse, ApiError> {
    let settings = state.services.settings.get_settings().await?;
    let loopback = match settings.bind_address {
        IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        IpAddr::V4(_) => IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
    };
    let mut addresses =
        if settings.bind_address.is_unspecified() || settings.bind_address.is_loopback() {
            vec![loopback]
        } else {
            vec![]
        };
    if settings.bind_address.is_unspecified()
        && let Ok(interfaces) = if_addrs::get_if_addrs()
    {
        for interface in interfaces {
            let address = interface.ip();
            if !address.is_loopback()
                && state.services.network_policy.allows(address)
                && !addresses.contains(&address)
            {
                addresses.push(address);
            }
        }
    } else if !settings.bind_address.is_loopback()
        && state.services.network_policy.allows(settings.bind_address)
    {
        addresses.push(settings.bind_address);
    }
    let urls = addresses
        .into_iter()
        .map(|address| {
            if address.is_ipv6() {
                format!("http://[{address}]:{}", settings.port)
            } else {
                format!("http://{address}:{}", settings.port)
            }
        })
        .collect();
    Ok(Json(NetworkInfo {
        bind_address: settings.bind_address,
        port: settings.port,
        urls,
        authentication: false,
        restart_required_after_bind_change: false,
        local_client: request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .is_some_and(|ConnectInfo(address)| is_host_client(address.ip())),
    }))
}

async fn logs(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.diagnostics.logs().await?))
}

async fn clear_logs(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    state.services.diagnostics.clear_logs().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn diagnostics(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.services.diagnostics.report().await?))
}

#[derive(Debug, Serialize)]
struct OperationAccepted {
    accepted: bool,
}

async fn runtime_install(
    State(state): State<AppState>,
    Path(component): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    start_runtime_action(state, component, RuntimeAction::Install).await
}

async fn runtime_update(
    State(state): State<AppState>,
    Path(component): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    start_runtime_action(state, component, RuntimeAction::Update).await
}

async fn runtime_repair(
    State(state): State<AppState>,
    Path(component): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    start_runtime_action(state, component, RuntimeAction::Repair).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeAction {
    Install,
    Update,
    Repair,
}

impl RuntimeAction {
    fn label(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Update => "update",
            Self::Repair => "repair",
        }
    }
}

async fn start_runtime_action(
    state: AppState,
    component: String,
    action: RuntimeAction,
) -> Result<impl IntoResponse, ApiError> {
    if !matches!(component.as_str(), "yt-dlp" | "ffmpeg" | "ffprobe") {
        return Err(ApiError(FetchError::InvalidRequest(
            "unknown runtime component".into(),
        )));
    }
    let manager = state.services.runtime.clone();
    let status = state.services.status.clone();
    let diagnostics = state.services.diagnostics.clone();
    tokio::spawn(async move {
        let action_label = action.label();
        let start_details =
            format!("component: {component}\naction: {action_label}\nsource: host request");
        let _ = diagnostics
            .record_log(
                "info",
                "runtime",
                "runtime operation started",
                Some(&start_details),
            )
            .await;
        let result = match (component.as_str(), action) {
            ("yt-dlp", RuntimeAction::Install) => manager.install_ytdlp().await.map(|_| ()),
            ("yt-dlp", RuntimeAction::Update) => manager.update_ytdlp().await.map(|_| ()),
            ("yt-dlp", RuntimeAction::Repair) => manager.repair_ytdlp().await.map(|_| ()),
            (_, RuntimeAction::Install) => manager.install_ffmpeg().await.map(|_| ()),
            (_, RuntimeAction::Update) => manager.update_ffmpeg().await.map(|_| ()),
            (_, RuntimeAction::Repair) => manager.repair_ffmpeg().await.map(|_| ()),
        };
        let components = manager.components().await;
        status.set_runtime_ready(
            components
                .iter()
                .all(|component| component.status == RuntimeStatus::Ready),
        );
        match result {
            Ok(()) => {
                let details =
                    format!("component: {component}\naction: {action_label}\nresult: completed");
                let _ = diagnostics
                    .record_log(
                        "info",
                        "runtime",
                        "runtime operation completed",
                        Some(&details),
                    )
                    .await;
            }
            Err(error) => {
                let retained = action == RuntimeAction::Update
                    && components.iter().any(|item| {
                        item.name.executable_name() == component
                            && item.status == RuntimeStatus::Ready
                    });
                let level = if retained { "warn" } else { "error" };
                let message = if retained {
                    "runtime update failed; existing component remains ready"
                } else {
                    "runtime operation failed"
                };
                let cause = error
                    .diagnostic_details()
                    .map(str::to_owned)
                    .unwrap_or_else(|| error.public_message());
                let details = format!("component: {component}\naction: {action_label}\n{cause}");
                let _ = diagnostics
                    .record_log(level, "runtime", message, Some(&details))
                    .await;
            }
        }
    });
    Ok((
        StatusCode::ACCEPTED,
        Json(OperationAccepted { accepted: true }),
    ))
}

async fn events(
    State(state): State<AppState>,
    connection: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let mut runtime_events = state.services.runtime.subscribe();
    let mut application_events = state.services.events.subscribe();
    let shutdown = state.services.shutdown.clone();
    let host_client =
        connection.is_some_and(|Extension(ConnectInfo(address))| is_host_client(address.ip()));
    let stream = async_stream::stream! {
        // Flush the stream immediately, including when no application events occur.
        yield Ok(Event::default().comment("connected"));
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                runtime = runtime_events.recv() => match runtime {
                    Ok(component) => {
                    let event_name = match component.status {
                        RuntimeStatus::Installing => "runtime.installing",
                        RuntimeStatus::Updating => "runtime.updating",
                        RuntimeStatus::Ready => "runtime.ready",
                        RuntimeStatus::Failed => "runtime.failed",
                        RuntimeStatus::Missing => "runtime.missing",
                    };
                    if let Ok(event) = Event::default().event(event_name).json_data(component) {
                        yield Ok(event);
                    }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
                application = application_events.recv() => match application {
                    Ok(application) => {
                        let event = if let Some(job) = application.job() {
                            Event::default().event(application.event_name()).json_data(job)
                        } else if let Some(file) = application.completed_file() {
                            Event::default().event(application.event_name()).json_data(file)
                        } else if let Some(progress) = application.playback_progress() {
                            Event::default().event(application.event_name()).json_data(progress)
                        } else if let Some(id) = application.cleared_playback_file() {
                            Event::default().event(application.event_name()).json_data(id)
                        } else if let ApplicationEvent::ProcessingUpdated(job) = &application {
                            Event::default().event(application.event_name()).json_data(job)
                        } else if let ApplicationEvent::MetadataSaved(status) = &application {
                            Event::default().event(application.event_name()).json_data(status)
                        } else if let Some(status) = application.telegram_status() {
                            if !host_client {
                                continue;
                            }
                            Event::default().event(application.event_name()).json_data(status)
                        } else {
                            continue;
                        };
                        if let Ok(event) = event {
                            yield Ok(event);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
            }
        }
    };
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            ErrorCode::NotFound,
            "The requested API endpoint does not exist",
        )),
    )
}

struct ApiError(FetchError);

impl From<FetchError> for ApiError {
    fn from(value: FetchError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            FetchError::UnsupportedUrl
            | FetchError::InvalidRequest(_)
            | FetchError::InvalidSettings(_)
            | FetchError::OutputDirectoryUnavailable(_) => StatusCode::BAD_REQUEST,
            FetchError::AuthenticationRequired => StatusCode::UNAUTHORIZED,
            FetchError::GeoRestricted | FetchError::MediaUnavailable => {
                StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS
            }
            FetchError::RuntimeMissing(_) | FetchError::RuntimeCorrupt(_) => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            FetchError::FileNotFound | FetchError::NotFound => StatusCode::NOT_FOUND,
            FetchError::NetworkDenied => StatusCode::FORBIDDEN,
            FetchError::LocalClientRequired => StatusCode::FORBIDDEN,
            FetchError::Conflict(_) => StatusCode::CONFLICT,
            FetchError::InvalidTransition { .. } => StatusCode::CONFLICT,
            FetchError::ProcessFailed { .. } | FetchError::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let response = ErrorResponse {
            error: fetch_core::ErrorBody {
                code: self.0.code(),
                message: self.0.public_message(),
                details: None,
            },
        };
        (status, Json(response)).into_response()
    }
}

async fn spa(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let asset_path = if path.is_empty() { "index.html" } else { path };
    if let Some(response) = embedded_asset(asset_path) {
        return response;
    }
    if !asset_path
        .rsplit('/')
        .next()
        .is_some_and(|part| part.contains('.'))
        && let Some(response) = embedded_asset("index.html")
    {
        return response;
    }
    (
        StatusCode::NOT_FOUND,
        "Embedded web UI was not found. Build web/ before compiling Fetch.",
    )
        .into_response()
}

fn embedded_asset(path: &str) -> Option<Response> {
    let asset = WebAssets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    // The entry document must always be revalidated so a browser cannot keep an
    // old asset manifest after Fetch is upgraded. Vite fingerprints files in
    // assets/, making those safe to cache for the lifetime of their URL.
    let cache_control = if path == "index.html" {
        "no-cache, no-store, must-revalidate"
    } else if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
            .header(header::CACHE_CONTROL, cache_control)
            .body(Body::from(asset.data.into_owned()))
            .expect("static asset response headers are valid"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use fetch_core::{DownloadMode, MediaInfo, MediaKind, TelegramConnectionState, TelegramStatus};
    use fetch_runtime::RuntimePaths;
    use futures::StreamExt;
    use tower::ServiceExt;

    struct TestMedia;
    struct TestDownloads;
    struct TestCompleted;
    struct TestCompletedFile(fetch_core::CompletedFile);
    struct TestSettings;
    struct TestProxy;
    struct TestListener;
    struct TestDiagnostics;
    struct TestTelegram;

    #[async_trait::async_trait]
    impl MediaAnalysis for TestMedia {
        async fn analyze(&self, _url: &str) -> Result<MediaInfo, FetchError> {
            Ok(MediaInfo {
                kind: MediaKind::Media,
                id: Some("fixture".into()),
                extractor: Some("Fixture".into()),
                title: "Fixture".into(),
                webpage_url: None,
                duration_seconds: None,
                thumbnail_url: None,
                playlist_count: None,
                entries: vec![],
                formats: vec![],
            })
        }
    }

    #[async_trait::async_trait]
    impl DownloadOperations for TestDownloads {
        async fn create(
            &self,
            _request: DownloadRequest,
        ) -> Result<fetch_core::DownloadJob, FetchError> {
            Err(FetchError::InvalidRequest(
                "test service does not create jobs".into(),
            ))
        }
        async fn list(&self) -> Result<Vec<fetch_core::DownloadJob>, FetchError> {
            Ok(vec![])
        }
        async fn get(&self, _id: uuid::Uuid) -> Result<fetch_core::DownloadJob, FetchError> {
            Err(FetchError::NotFound)
        }
        async fn stop(&self, id: uuid::Uuid) -> Result<fetch_core::DownloadJob, FetchError> {
            self.get(id).await
        }
        async fn resume(&self, id: uuid::Uuid) -> Result<fetch_core::DownloadJob, FetchError> {
            self.get(id).await
        }
        async fn retry(&self, id: uuid::Uuid) -> Result<fetch_core::DownloadJob, FetchError> {
            self.get(id).await
        }
        async fn delete(&self, _id: uuid::Uuid) -> Result<(), FetchError> {
            Err(FetchError::NotFound)
        }
        async fn update_defaults(
            &self,
            _download_directory: std::path::PathBuf,
            _concurrent_downloads: u8,
            _ytdlp_js_runtime: fetch_core::YtDlpJsRuntime,
        ) -> Result<(), FetchError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl CompletedOperations for TestCompleted {
        async fn list_completed(&self) -> Result<Vec<fetch_core::CompletedFile>, FetchError> {
            Ok(vec![])
        }
        async fn get_completed(
            &self,
            _id: uuid::Uuid,
        ) -> Result<fetch_core::CompletedFile, FetchError> {
            Err(FetchError::FileNotFound)
        }
        async fn reveal_completed(&self, _id: uuid::Uuid) -> Result<(), FetchError> {
            Err(FetchError::FileNotFound)
        }
        async fn delete_completed(&self, _id: uuid::Uuid) -> Result<(), FetchError> {
            Err(FetchError::FileNotFound)
        }
        async fn save_playback_progress(
            &self,
            _id: uuid::Uuid,
            _update: fetch_core::PlaybackProgressUpdate,
        ) -> Result<fetch_core::PlaybackProgress, FetchError> {
            Err(FetchError::FileNotFound)
        }
        async fn clear_playback_progress(&self, _id: uuid::Uuid) -> Result<(), FetchError> {
            Err(FetchError::FileNotFound)
        }
    }

    #[async_trait::async_trait]
    impl CompletedOperations for TestCompletedFile {
        async fn start_export(
            &self,
            id: uuid::Uuid,
            request: fetch_core::ExportRequest,
        ) -> Result<fetch_core::ProcessingJob, FetchError> {
            self.get_completed(id).await?;
            request.validate()?;
            if request.revision != "v1" {
                return Err(FetchError::Conflict("File changed".into()));
            }
            Ok(fetch_core::ProcessingJob::new(
                request.kind(),
                id,
                self.0.filename.clone(),
            ))
        }

        async fn metadata(&self, id: uuid::Uuid) -> Result<fetch_core::MediaMetadata, FetchError> {
            self.get_completed(id).await?;
            Ok(fetch_core::MediaMetadata {
                revision: "v1".into(),
                editable: true,
                media_type: "video".into(),
                container: "mp4".into(),
                fields: std::collections::BTreeMap::from([("title".into(), "Fixture".into())]),
                supported_fields: vec!["title".into()],
                artwork_available: false,
                artwork_editable: true,
                information: Default::default(),
            })
        }
        async fn update_metadata(
            &self,
            id: uuid::Uuid,
            update: fetch_core::MetadataUpdate,
        ) -> Result<fetch_core::MetadataSaveStatus, FetchError> {
            self.get_completed(id).await?;
            update.validate()?;
            if update.revision != "v1" {
                return Err(FetchError::Conflict("File changed".into()));
            }
            Ok(fetch_core::MetadataSaveStatus {
                file_id: id,
                operation_id: Some(uuid::Uuid::new_v4()),
                state: fetch_core::MetadataSaveState::Saving,
                error: None,
            })
        }
        async fn metadata_status(
            &self,
            id: uuid::Uuid,
        ) -> Result<fetch_core::MetadataSaveStatus, FetchError> {
            self.get_completed(id).await?;
            Ok(fetch_core::MetadataSaveStatus {
                file_id: id,
                operation_id: None,
                state: fetch_core::MetadataSaveState::Idle,
                error: None,
            })
        }

        async fn list_completed(&self) -> Result<Vec<fetch_core::CompletedFile>, FetchError> {
            Ok(vec![self.0.clone()])
        }
        async fn get_completed(
            &self,
            id: uuid::Uuid,
        ) -> Result<fetch_core::CompletedFile, FetchError> {
            (self.0.id == id)
                .then(|| self.0.clone())
                .ok_or(FetchError::FileNotFound)
        }
        async fn reveal_completed(&self, id: uuid::Uuid) -> Result<(), FetchError> {
            (self.0.id == id)
                .then_some(())
                .ok_or(FetchError::FileNotFound)
        }
        async fn delete_completed(&self, id: uuid::Uuid) -> Result<(), FetchError> {
            (self.0.id == id)
                .then_some(())
                .ok_or(FetchError::FileNotFound)
        }
        async fn save_playback_progress(
            &self,
            id: uuid::Uuid,
            update: fetch_core::PlaybackProgressUpdate,
        ) -> Result<fetch_core::PlaybackProgress, FetchError> {
            self.get_completed(id).await?;
            Ok(fetch_core::PlaybackProgress {
                file_id: id,
                position_seconds: update.position_seconds,
                duration_seconds: update.duration_seconds,
                completed: false,
                updated_at: chrono::Utc::now(),
            })
        }
        async fn clear_playback_progress(&self, id: uuid::Uuid) -> Result<(), FetchError> {
            self.get_completed(id).await.map(|_| ())
        }
    }

    #[async_trait::async_trait]
    impl SettingsOperations for TestSettings {
        async fn get_settings(&self) -> Result<ApplicationSettings, FetchError> {
            Ok(ApplicationSettings {
                bind_address: "127.0.0.1".parse().unwrap(),
                port: 8080,
                allowed_networks: vec!["192.168.0.0/16".into()],
                download_directory: "downloads".into(),
                concurrent_downloads: 3,
                open_browser_on_start: true,
                start_with_system: false,
                ytdlp_auto_update: true,
                ytdlp_js_runtime: fetch_core::YtDlpJsRuntime::Auto,
            })
        }
        async fn put_settings(
            &self,
            settings: ApplicationSettings,
        ) -> Result<ApplicationSettings, FetchError> {
            Ok(settings)
        }
    }

    #[async_trait::async_trait]
    impl ProxyOperations for TestProxy {
        async fn get_proxy(&self) -> Result<ProxySettings, FetchError> {
            Ok(ProxySettings::default())
        }

        async fn put_proxy(&self, settings: ProxySettings) -> Result<ProxySettings, FetchError> {
            settings.validate()?;
            Ok(settings)
        }
    }

    #[async_trait::async_trait]
    impl TelegramOperations for TestTelegram {
        async fn get_integration(&self) -> Result<TelegramIntegration, FetchError> {
            Ok(TelegramIntegration {
                settings: TelegramSettings::default(),
                status: fetch_core::TelegramStatus::default(),
            })
        }

        async fn put_settings(
            &self,
            settings: TelegramSettings,
        ) -> Result<TelegramIntegration, FetchError> {
            settings.validate()?;
            Ok(TelegramIntegration {
                settings,
                status: fetch_core::TelegramStatus::default(),
            })
        }

        async fn put_token(
            &self,
            _token: TelegramToken,
        ) -> Result<TelegramIntegration, FetchError> {
            self.get_integration().await
        }

        async fn delete_token(&self) -> Result<TelegramIntegration, FetchError> {
            self.get_integration().await
        }

        async fn restart(&self) -> Result<TelegramIntegration, FetchError> {
            self.get_integration().await
        }

        async fn test_connection(&self) -> Result<TelegramIntegration, FetchError> {
            self.get_integration().await
        }
    }

    #[async_trait::async_trait]
    impl ListenerOperations for TestListener {
        async fn rebind(&self, _address: SocketAddr) -> Result<bool, FetchError> {
            Ok(false)
        }
    }

    #[async_trait::async_trait]
    impl DiagnosticOperations for TestDiagnostics {
        async fn logs(&self) -> Result<Vec<fetch_core::DiagnosticLogEntry>, FetchError> {
            Ok(vec![])
        }
        async fn clear_logs(&self) -> Result<(), FetchError> {
            Ok(())
        }
        async fn report(&self) -> Result<fetch_core::DiagnosticsReport, FetchError> {
            Ok(fetch_core::DiagnosticsReport { checks: vec![] })
        }
        async fn record_log(
            &self,
            _level: &str,
            _subsystem: &str,
            _message: &str,
            _details: Option<&str>,
        ) -> Result<(), FetchError> {
            Ok(())
        }
    }

    fn app() -> Router {
        app_with_completed(Arc::new(TestCompleted))
    }

    fn app_with_completed(completed: Arc<dyn CompletedOperations>) -> Router {
        app_with_shutdown(completed, CancellationToken::new())
    }

    fn app_with_shutdown(
        completed: Arc<dyn CompletedOperations>,
        shutdown: CancellationToken,
    ) -> Router {
        app_with_event_bus(completed, shutdown, EventBus::default())
    }

    fn app_with_event_bus(
        completed: Arc<dyn CompletedOperations>,
        shutdown: CancellationToken,
        events: EventBus,
    ) -> Router {
        router(ServerServices {
            status: StatusService::new("0.1.0", true, false),
            runtime: RuntimeManager::new(RuntimePaths::new("test-runtime")).unwrap(),
            media: Arc::new(TestMedia),
            downloads: Arc::new(TestDownloads),
            events,
            completed,
            settings: Arc::new(TestSettings),
            proxy: Arc::new(TestProxy),
            listener: Arc::new(TestListener),
            network_policy: NetworkPolicy::new(&["192.168.0.0/16".into()]).unwrap(),
            diagnostics: Arc::new(TestDiagnostics),
            telegram: Arc::new(TestTelegram),
            shutdown,
        })
    }

    #[tokio::test]
    async fn status_is_typed_json() {
        let response = request(app(), "/api/status", "GET", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["server"], "ready");
        assert_eq!(json["storage_ready"], true);
        assert_eq!(json["runtime_ready"], false);
    }

    #[tokio::test]
    async fn javascript_runtime_status_has_stable_typed_entries() {
        let response = request(app(), "/api/runtime/javascript", "GET", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 3);
        assert_eq!(json[0]["name"], "deno");
        assert_eq!(json[1]["name"], "node");
        assert_eq!(json[2]["name"], "quickjs");
        assert!(json[1]["detected"].is_boolean());
    }

    #[tokio::test]
    async fn media_analysis_uses_application_service() {
        let response = request(
            app(),
            "/api/media/analyze",
            "POST",
            Some(r#"{"url":"https://example.test/media"}"#),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["title"],
            "Fixture"
        );
    }

    #[tokio::test]
    async fn unknown_api_path_returns_json_and_never_spa() {
        let response = request(app(), "/api/not-real", "GET", None).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    }

    #[tokio::test]
    async fn client_route_uses_embedded_spa() {
        let response = request(app(), "/settings", "GET", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-cache, no-store, must-revalidate"
        );
    }

    #[test]
    fn fingerprinted_frontend_assets_are_immutable() {
        let path = WebAssets::iter()
            .find(|path| path.starts_with("assets/") && path.ends_with(".js"))
            .expect("the embedded frontend includes a fingerprinted JavaScript asset");
        let response = embedded_asset(path.as_ref()).expect("embedded asset exists");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn shutdown_cancellation_closes_active_event_streams() {
        let shutdown = CancellationToken::new();
        let response = request(
            app_with_shutdown(Arc::new(TestCompleted), shutdown.clone()),
            "/api/events",
            "GET",
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/event-stream")
        );

        let body = tokio::spawn(to_bytes(response.into_body(), usize::MAX));
        tokio::task::yield_now().await;
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(1), body)
            .await
            .expect("SSE body did not close after shutdown cancellation")
            .expect("SSE body task panicked")
            .expect("SSE body returned an error");
    }

    #[tokio::test]
    async fn idle_event_stream_sends_an_immediate_opening_comment() {
        let response = request(app(), "/api/events", "GET", None).await;
        let mut body = response.into_body().into_data_stream();
        let first = tokio::time::timeout(Duration::from_millis(250), body.next())
            .await
            .expect("idle SSE must not wait for an event or the 15-second keepalive")
            .unwrap()
            .unwrap();
        assert_eq!(&first[..], b": connected\n\n");
    }

    #[tokio::test]
    async fn telegram_status_events_are_visible_to_host_clients_only() {
        let host_events = EventBus::default();
        let host_shutdown = CancellationToken::new();
        let host_app = app_with_event_bus(
            Arc::new(TestCompleted),
            host_shutdown.clone(),
            host_events.clone(),
        );
        let host = request_from(
            host_app.clone(),
            "/api/events",
            "GET",
            ([127, 0, 0, 1], 4000).into(),
        )
        .await;
        host_events.publish(ApplicationEvent::TelegramStatusUpdated(TelegramStatus {
            state: TelegramConnectionState::Connected,
            ..TelegramStatus::default()
        }));
        let mut host_body = host.into_body().into_data_stream();
        let opening = host_body.next().await.unwrap().unwrap();
        assert_eq!(&opening[..], b": connected\n\n");
        let host_chunk = tokio::time::timeout(Duration::from_secs(1), host_body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(String::from_utf8_lossy(&host_chunk).contains("event: telegram.status"));
        host_shutdown.cancel();

        let remote_events = EventBus::default();
        let remote_shutdown = CancellationToken::new();
        let remote_app = app_with_event_bus(
            Arc::new(TestCompleted),
            remote_shutdown.clone(),
            remote_events.clone(),
        );
        let remote = request_from(
            remote_app.clone(),
            "/api/events",
            "GET",
            ([192, 168, 2, 8], 4000).into(),
        )
        .await;
        assert_eq!(remote.status(), StatusCode::OK);
        remote_events.publish(ApplicationEvent::TelegramStatusUpdated(TelegramStatus {
            state: TelegramConnectionState::Connected,
            ..TelegramStatus::default()
        }));
        remote_events.publish(ApplicationEvent::DownloadCreated(
            fetch_core::DownloadJob::new(DownloadRequest {
                url: "https://example.test/media".into(),
                title: None,
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
            }),
        ));
        let mut remote_body = remote.into_body().into_data_stream();
        let opening = remote_body.next().await.unwrap().unwrap();
        assert_eq!(&opening[..], b": connected\n\n");
        let remote_chunk = tokio::time::timeout(Duration::from_secs(1), remote_body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let remote_text = String::from_utf8_lossy(&remote_chunk);
        assert!(remote_text.contains("event: download.created"));
        assert!(!remote_text.contains("telegram.status"));
        remote_shutdown.cancel();
    }

    #[tokio::test]
    async fn invalid_network_settings_return_a_typed_error() {
        let response = request(
            app(),
            "/api/settings",
            "PUT",
            Some(r#"{"bind_address":"127.0.0.1","port":8080,"allowed_networks":["not-a-cidr"],"download_directory":"downloads","concurrent_downloads":3,"open_browser_on_start":false,"start_with_system":false,"ytdlp_auto_update":true}"#),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
            "INVALID_SETTINGS"
        );
    }

    #[tokio::test]
    async fn remote_clients_cannot_change_host_startup_registration() {
        let mut request = axum::http::Request::builder()
            .uri("/api/settings")
            .method("PUT")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"bind_address":"127.0.0.1","port":8080,"allowed_networks":["192.168.0.0/16"],"download_directory":"downloads","concurrent_downloads":3,"open_browser_on_start":true,"start_with_system":true,"ytdlp_auto_update":true}"#))
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([192, 168, 2, 8], 4000))));
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
            "LOCAL_CLIENT_REQUIRED"
        );
    }

    #[tokio::test]
    async fn proxy_configuration_is_host_only_and_validated() {
        let local = request_json_from(
            app(),
            "/api/proxy",
            "PUT",
            r#"{"mode":"custom","url":"socks5://127.0.0.1:1080"}"#,
            ([127, 0, 0, 1], 4000).into(),
        )
        .await;
        assert_eq!(local.status(), StatusCode::OK);
        let body = to_bytes(local.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["mode"], "custom");

        let invalid = request_json_from(
            app(),
            "/api/proxy",
            "PUT",
            r#"{"mode":"custom","url":"http://user:secret@proxy.test:8080"}"#,
            ([127, 0, 0, 1], 4000).into(),
        )
        .await;
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(invalid.into_body(), usize::MAX).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("secret"));

        for method in ["GET", "PUT"] {
            let remote = request_json_from(
                app(),
                "/api/proxy",
                method,
                r#"{"mode":"direct","url":null}"#,
                ([192, 168, 2, 8], 4000).into(),
            )
            .await;
            assert_eq!(remote.status(), StatusCode::FORBIDDEN);
        }
    }

    #[tokio::test]
    async fn telegram_configuration_is_host_only_and_never_returns_a_token() {
        let local =
            request_from(app(), "/api/telegram", "GET", ([127, 0, 0, 1], 4000).into()).await;
        assert_eq!(local.status(), StatusCode::OK);
        let body = to_bytes(local.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("settings").is_some());
        assert!(json.get("status").is_some());
        assert!(!String::from_utf8_lossy(&body).contains("bot-token"));

        let remote = request_from(
            app(),
            "/api/telegram",
            "GET",
            ([192, 168, 2, 8], 4000).into(),
        )
        .await;
        assert_eq!(remote.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn telegram_restart_is_host_only() {
        let local = request_from(
            app(),
            "/api/telegram/restart",
            "POST",
            ([127, 0, 0, 1], 4000).into(),
        )
        .await;
        assert_eq!(local.status(), StatusCode::OK);
        let remote = request_from(
            app(),
            "/api/telegram/restart",
            "POST",
            ([192, 168, 2, 8], 4000).into(),
        )
        .await;
        assert_eq!(remote.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn process_errors_do_not_expose_raw_diagnostics() {
        let response = ApiError(FetchError::ProcessFailed {
            summary: "Download failed".into(),
            details: "private process output and stack".into(),
        })
        .into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload = serde_json::from_slice::<serde_json::Value>(&body).unwrap();
        assert_eq!(payload["error"]["message"], "Download failed");
        assert!(payload["error"]["details"].is_null());
        assert!(!String::from_utf8_lossy(&body).contains("private process output"));
    }

    #[tokio::test]
    async fn remote_clients_outside_the_allow_list_are_rejected() {
        let mut denied = axum::http::Request::builder()
            .uri("/api/status")
            .body(Body::empty())
            .unwrap();
        denied
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([10, 0, 0, 2], 4000))));
        let response = app().oneshot(denied).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let mut allowed = axum::http::Request::builder()
            .uri("/api/status")
            .body(Body::empty())
            .unwrap();
        allowed
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([192, 168, 2, 8], 4000))));
        assert_eq!(
            app().oneshot(allowed).await.unwrap().status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn opaque_file_endpoint_streams_exact_ranges() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("media.mp4");
        tokio::fs::write(&path, b"0123456789").await.unwrap();
        let file = fetch_core::CompletedFile {
            id: uuid::Uuid::new_v4(),
            job_id: Some(uuid::Uuid::new_v4()),
            origin: None,
            source_file_id: None,
            playlist: None,
            filename: "media.mp4".into(),
            path,
            thumbnail_path: None,
            thumbnail_available: false,
            size_bytes: 10,
            mime_type: "video/mp4".into(),
            title: Some("Media".into()),
            browser_playable: true,
            playback: None,
            created_at: chrono::Utc::now(),
        };
        let id = file.id;
        let mut range_request = axum::http::Request::builder()
            .uri(format!("/api/files/{id}/stream"))
            .header(header::RANGE, "bytes=2-5")
            .body(Body::empty())
            .unwrap();
        range_request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 4000))));
        let response = app_with_completed(Arc::new(TestCompletedFile(file)))
            .oneshot(range_request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 2-5/10");
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            "2345"
        );
    }

    #[tokio::test]
    async fn opaque_thumbnail_endpoint_serves_cached_artwork() {
        let directory = tempfile::tempdir().unwrap();
        let media_path = directory.path().join("media.mp4");
        let thumbnail_path = directory.path().join("thumbnail.jpg");
        tokio::fs::write(&media_path, b"media").await.unwrap();
        tokio::fs::write(&thumbnail_path, b"jpeg-bytes")
            .await
            .unwrap();
        let file = fetch_core::CompletedFile {
            id: uuid::Uuid::new_v4(),
            job_id: Some(uuid::Uuid::new_v4()),
            origin: None,
            source_file_id: None,
            playlist: None,
            filename: "media.mp4".into(),
            path: media_path,
            thumbnail_path: Some(thumbnail_path),
            thumbnail_available: true,
            size_bytes: 5,
            mime_type: "video/mp4".into(),
            title: Some("Media".into()),
            browser_playable: true,
            playback: None,
            created_at: chrono::Utc::now(),
        };
        let id = file.id;
        let response = request(
            app_with_completed(Arc::new(TestCompletedFile(file))),
            &format!("/api/files/{id}/thumbnail"),
            "GET",
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            "jpeg-bytes"
        );
    }

    #[tokio::test]
    async fn reveal_is_loopback_only_while_completed_delete_is_available_to_allowed_clients() {
        let file = fetch_core::CompletedFile {
            id: uuid::Uuid::new_v4(),
            job_id: Some(uuid::Uuid::new_v4()),
            origin: None,
            source_file_id: None,
            playlist: None,
            filename: "media.mp4".into(),
            path: "media.mp4".into(),
            thumbnail_path: None,
            thumbnail_available: false,
            size_bytes: 5,
            mime_type: "video/mp4".into(),
            title: Some("Media".into()),
            browser_playable: true,
            playback: None,
            created_at: chrono::Utc::now(),
        };
        let id = file.id;
        let completed: Arc<dyn CompletedOperations> = Arc::new(TestCompletedFile(file));

        let local = request_from(
            app_with_completed(completed.clone()),
            &format!("/api/files/{id}/reveal"),
            "POST",
            ([127, 0, 0, 1], 4000).into(),
        )
        .await;
        assert_eq!(local.status(), StatusCode::NO_CONTENT);

        let remote = request_from(
            app_with_completed(completed.clone()),
            &format!("/api/files/{id}/reveal"),
            "POST",
            ([192, 168, 2, 8], 4000).into(),
        )
        .await;
        assert_eq!(remote.status(), StatusCode::FORBIDDEN);

        let delete = request_from(
            app_with_completed(completed),
            &format!("/api/files/{id}"),
            "DELETE",
            ([192, 168, 2, 8], 4000).into(),
        )
        .await;
        assert_eq!(delete.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn playback_progress_can_be_saved_and_cleared_by_opaque_file_id() {
        let file = fetch_core::CompletedFile {
            id: uuid::Uuid::new_v4(),
            job_id: Some(uuid::Uuid::new_v4()),
            origin: None,
            source_file_id: None,
            playlist: None,
            filename: "media.mp4".into(),
            path: "media.mp4".into(),
            thumbnail_path: None,
            thumbnail_available: false,
            size_bytes: 5,
            mime_type: "video/mp4".into(),
            title: Some("Media".into()),
            browser_playable: true,
            playback: None,
            created_at: chrono::Utc::now(),
        };
        let id = file.id;
        let completed: Arc<dyn CompletedOperations> = Arc::new(TestCompletedFile(file));
        let saved = request(
            app_with_completed(completed.clone()),
            &format!("/api/files/{id}/progress"),
            "PUT",
            Some(r#"{"position_seconds":30.0,"duration_seconds":100.0}"#),
        )
        .await;
        assert_eq!(saved.status(), StatusCode::OK);
        let body = to_bytes(saved.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["position_seconds"],
            30.0
        );
        let cleared = request(
            app_with_completed(completed),
            &format!("/api/files/{id}/progress"),
            "DELETE",
            None,
        )
        .await;
        assert_eq!(cleared.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn network_info_identifies_the_host_browser() {
        let local = request_from(app(), "/api/network", "GET", ([127, 0, 0, 1], 4000).into()).await;
        let body = to_bytes(local.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["local_client"],
            true
        );

        let remote = request_from(
            app(),
            "/api/network",
            "GET",
            ([192, 168, 2, 8], 4000).into(),
        )
        .await;
        let body = to_bytes(remote.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap()["local_client"],
            false
        );
    }

    async fn request(app: Router, uri: &str, method: &str, body: Option<&str>) -> Response {
        app.oneshot(
            axum::http::Request::builder()
                .uri(uri)
                .method(method)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.unwrap_or_default().to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn request_from(app: Router, uri: &str, method: &str, address: SocketAddr) -> Response {
        let mut request = axum::http::Request::builder()
            .uri(uri)
            .method(method)
            .body(Body::empty())
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(address));
        app.oneshot(request).await.unwrap()
    }

    async fn request_json_from(
        app: Router,
        uri: &str,
        method: &str,
        body: &str,
        address: SocketAddr,
    ) -> Response {
        let mut request = axum::http::Request::builder()
            .uri(uri)
            .method(method)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(address));
        app.oneshot(request).await.unwrap()
    }

    #[test]
    fn range_parser_handles_bounded_open_and_suffix_ranges() {
        assert_eq!(parse_range("bytes=2-5", 10), Some((2, 5)));
        assert_eq!(parse_range("bytes=7-", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=10-", 10), None);
        assert_eq!(parse_range("bytes=1-2,4-5", 10), None);
    }

    #[test]
    fn network_policy_allows_loopback_and_configured_cidr_only() {
        let policy = NetworkPolicy::new(&["192.168.0.0/16".into()]).unwrap();
        assert!(policy.allows("127.0.0.1".parse().unwrap()));
        assert!(policy.allows("192.168.4.20".parse().unwrap()));
        assert!(!policy.allows("10.0.0.2".parse().unwrap()));
        assert!(NetworkPolicy::new(&["invalid".into()]).is_err());
    }
    #[tokio::test]
    async fn metadata_api_resolves_opaque_ids_and_validates_background_requests() {
        let file = fetch_core::CompletedFile {
            id: uuid::Uuid::new_v4(),
            job_id: Some(uuid::Uuid::new_v4()),
            origin: None,
            source_file_id: None,
            playlist: None,
            filename: "fixture.mp4".into(),
            path: "/private/fixture.mp4".into(),
            thumbnail_path: None,
            thumbnail_available: false,
            size_bytes: 42,
            mime_type: "video/mp4".into(),
            title: Some("Fixture".into()),
            browser_playable: true,
            playback: None,
            created_at: chrono::Utc::now(),
        };
        let export_path = format!("/api/files/{}/processes", file.id);
        let path = format!("/api/files/{}/metadata", file.id);
        let app = app_with_completed(Arc::new(TestCompletedFile(file)));
        let export = r#"{"revision":"v1","filename":"Converted","format":"mp4","acknowledge_omissions":true}"#;
        let response = request(app.clone(), &export_path, "POST", Some(export)).await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let job: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(job["state"], "queued");
        assert_eq!(job["kind"], "conversion");
        assert!(!String::from_utf8_lossy(&body).contains("/private"));
        assert_eq!(
            request(
                app.clone(),
                &export_path,
                "POST",
                Some(&export.replace("Converted", "../unsafe"))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            request(
                app.clone(),
                &export_path,
                "POST",
                Some(&export.replace("v1", "stale"))
            )
            .await
            .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            request(
                app.clone(),
                "/api/files/not-a-uuid/processes",
                "POST",
                Some(export)
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            request(
                app.clone(),
                &export_path,
                "POST",
                Some(&export.replace("\"format\"", "\"args\""))
            )
            .await
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let response = request(app.clone(), &path, "GET", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("/private"));
        let valid = r#"{"revision":"v1","fields":{"title":"Changed"},"artwork":{"action":"keep"}}"#;
        let response = request(app.clone(), &path, "PUT", Some(valid)).await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["state"], "saving");
        assert!(body["operation_id"].is_string());
        let response = request(
            app.clone(),
            &path,
            "PUT",
            Some(&valid.replace("v1", "stale")),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let response = request(
            app.clone(),
            &path,
            "PUT",
            Some(&valid.replace("title", "path")),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = request(app.clone(), "/api/files/not-a-uuid/metadata", "GET", None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = request(
            app.clone(),
            &format!("/api/files/{}/metadata", uuid::Uuid::new_v4()),
            "GET",
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = request(app, &format!("{path}/status"), "GET", None).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
