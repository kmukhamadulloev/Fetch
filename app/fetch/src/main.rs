mod config;
mod diagnostics;
mod downloads;
mod library;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use config::FetchConfig;
use diagnostics::DiagnosticsService;
use downloads::DownloadManager;
use fetch_core::{
    ApplicationSettings, EventBus, FetchError, ListenerOperations, MediaAnalysis, MediaInfo,
    RuntimeStatus, StatusService,
};
use fetch_runtime::{RuntimeManager, RuntimePaths};
use fetch_storage::Storage;
use library::CompletedLibrary;
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const GRACEFUL_SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = FetchConfig::load().context("could not load Fetch configuration")?;
    init_tracing(&config.log_filter)?;

    let database_path = config.database_path();
    let storage = Arc::new(
        Storage::open(&database_path)
            .await
            .with_context(|| format!("could not initialize {}", database_path.display()))?,
    );
    storage
        .health_check()
        .await
        .context("SQLite health check failed after migration")?;

    let recovered = storage.recover_interrupted_jobs().await?;
    if recovered > 0 {
        warn!(recovered, "recovered interrupted download jobs as stopped");
    }
    let settings = match storage.load_settings().await? {
        Some(settings) => settings,
        None => {
            let settings = ApplicationSettings {
                bind_address: config.bind_address,
                port: config.port,
                allowed_networks: config.allowed_networks.clone(),
                download_directory: config.download_directory.clone(),
                concurrent_downloads: config.concurrent_downloads.clamp(1, 16) as u8,
                open_browser_on_start: config.open_browser_on_start,
                ytdlp_auto_update: config.ytdlp_auto_update,
            };
            settings.validate_basic()?;
            storage.save_settings(&settings).await?;
            settings
        }
    };
    let runtime = RuntimeManager::new(RuntimePaths::new(config.runtime_path()))?;
    let components = runtime.inspect_all().await;
    let runtime_ready = components
        .iter()
        .all(|component| component.status == RuntimeStatus::Ready);
    let status = StatusService::new(env!("CARGO_PKG_VERSION"), true, runtime_ready);
    let events = EventBus::default();
    let downloads = Arc::new(DownloadManager::new(
        storage.clone(),
        runtime.clone(),
        events.clone(),
        settings.concurrent_downloads as usize,
        settings.download_directory.clone(),
        config.data_directory.join("thumbnails"),
    ));
    let media = Arc::new(DynamicMediaAnalyzer {
        runtime: runtime.clone(),
    });
    let network_policy = fetch_server::NetworkPolicy::new(&settings.allowed_networks)?;
    let shutdown = CancellationToken::new();
    let address = SocketAddr::new(settings.bind_address, settings.port);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("could not bind HTTP server to {address}"))?;
    let (rebind_sender, rebind_receiver) = mpsc::channel(4);
    let listener_control = Arc::new(ListenerControl {
        current: Arc::new(RwLock::new(address)),
        sender: rebind_sender,
    });
    let app = fetch_server::router(fetch_server::ServerServices {
        status: status.clone(),
        runtime: runtime.clone(),
        media,
        downloads,
        events,
        completed: Arc::new(CompletedLibrary::new(storage.clone())),
        settings: storage.clone(),
        listener: listener_control,
        network_policy,
        diagnostics: Arc::new(DiagnosticsService::new(storage.clone(), runtime.clone())),
        shutdown: shutdown.clone(),
    });
    start_runtime_bootstrap(runtime, status, storage, settings.clone());

    info!(%address, database = %database_path.display(), "Fetch server ready");
    if !address.ip().is_loopback() {
        warn!(%address, "Fetch has no application authentication; restrict network access at the host boundary");
    }
    if settings.open_browser_on_start {
        let url = format!("http://127.0.0.1:{}/", address.port());
        if let Err(error) = webbrowser::open(&url) {
            warn!(%error, %url, "could not open the default browser");
        }
    }

    let tasks = Arc::new(Mutex::new(Vec::new()));
    let (server_error_sender, mut server_error_receiver) = mpsc::unbounded_channel();
    let initial_listener_shutdown = CancellationToken::new();
    spawn_http_server(
        listener,
        app.clone(),
        initial_listener_shutdown.clone(),
        shutdown.clone(),
        server_error_sender.clone(),
        tasks.clone(),
    )
    .await;
    tokio::spawn(rebind_listeners(
        rebind_receiver,
        app,
        initial_listener_shutdown,
        shutdown.clone(),
        server_error_sender,
        tasks.clone(),
    ));

    let unexpected_error = tokio::select! {
        () = shutdown_signal(shutdown.clone()) => None,
        error = server_error_receiver.recv() => error,
    };
    shutdown.cancel();
    let active_tasks = std::mem::take(&mut *tasks.lock().await);
    if tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, async {
        for task in active_tasks {
            let _ = task.await;
        }
    })
    .await
    .is_err()
    {
        warn!(
            timeout_seconds = GRACEFUL_SHUTDOWN_TIMEOUT.as_secs(),
            "graceful shutdown timed out; closing remaining connections"
        );
    }
    if let Some(error) = unexpected_error {
        return Err(anyhow::anyhow!(error).context("HTTP server stopped unexpectedly"));
    }
    Ok(())
}

struct RebindRequest {
    address: SocketAddr,
    response: oneshot::Sender<Result<(), String>>,
}

#[derive(Clone)]
struct ListenerControl {
    current: Arc<RwLock<SocketAddr>>,
    sender: mpsc::Sender<RebindRequest>,
}

#[async_trait::async_trait]
impl ListenerOperations for ListenerControl {
    async fn rebind(&self, address: SocketAddr) -> Result<bool, FetchError> {
        if *self.current.read().await == address {
            return Ok(false);
        }
        let (response, result) = oneshot::channel();
        self.sender
            .send(RebindRequest { address, response })
            .await
            .map_err(|_| FetchError::Internal("listener controller is unavailable".into()))?;
        result
            .await
            .map_err(|_| FetchError::Internal("listener controller stopped unexpectedly".into()))?
            .map_err(FetchError::InvalidSettings)?;
        *self.current.write().await = address;
        Ok(true)
    }
}

async fn spawn_http_server(
    listener: tokio::net::TcpListener,
    app: axum::Router,
    local_shutdown: CancellationToken,
    global_shutdown: CancellationToken,
    errors: mpsc::UnboundedSender<String>,
    tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
) {
    let task = tokio::spawn(async move {
        let server = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(global_shutdown.cancelled_owned());
        tokio::select! {
            result = server => if let Err(error) = result {
                let _ = errors.send(error.to_string());
            },
            () = local_shutdown.cancelled() => {}
        }
    });
    tasks.lock().await.push(task);
}

async fn rebind_listeners(
    mut requests: mpsc::Receiver<RebindRequest>,
    app: axum::Router,
    mut active_shutdown: CancellationToken,
    shutdown: CancellationToken,
    errors: mpsc::UnboundedSender<String>,
    tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
) {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            request = requests.recv() => {
                let Some(request) = request else { break };
                match tokio::net::TcpListener::bind(request.address).await {
                    Ok(listener) => {
                        let next_shutdown = CancellationToken::new();
                        spawn_http_server(listener, app.clone(), next_shutdown.clone(), shutdown.clone(), errors.clone(), tasks.clone()).await;
                        let previous_shutdown = std::mem::replace(&mut active_shutdown, next_shutdown);
                        let _ = request.response.send(Ok(()));
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            previous_shutdown.cancel();
                        });
                    }
                    Err(error) => {
                        let _ = request.response.send(Err(format!("could not bind Fetch to {}: {error}", request.address)));
                    }
                }
            }
        }
    }
}

fn start_runtime_bootstrap(
    runtime: RuntimeManager,
    status: StatusService,
    storage: Arc<Storage>,
    settings: ApplicationSettings,
) {
    tokio::spawn(async move {
        let components = runtime.components().await;
        let ytdlp = components
            .iter()
            .find(|component| component.name == fetch_core::RuntimeComponentName::YtDlp);
        let ffmpeg_missing = components.iter().any(|component| {
            matches!(
                component.name,
                fetch_core::RuntimeComponentName::Ffmpeg
                    | fetch_core::RuntimeComponentName::Ffprobe
            ) && component.status != RuntimeStatus::Ready
        });
        let last_update = storage
            .get_metadata("ytdlp_last_update")
            .await
            .ok()
            .flatten()
            .and_then(|value| value.parse::<chrono::DateTime<chrono::Utc>>().ok());
        let update_due = last_update
            .is_none_or(|updated| chrono::Utc::now() - updated > chrono::Duration::days(1));
        let ytdlp_result = if ytdlp.is_none_or(|component| component.status != RuntimeStatus::Ready)
        {
            runtime.install_ytdlp().await.map(|_| ())
        } else if settings.ytdlp_auto_update && update_due {
            runtime.update_ytdlp().await.map(|_| ())
        } else {
            Ok(())
        };
        if let Err(error) = &ytdlp_result {
            let _ = storage
                .append_log("error", "runtime", &error.public_message(), None)
                .await;
        } else if update_due {
            let _ = storage
                .set_metadata("ytdlp_last_update", &chrono::Utc::now().to_rfc3339())
                .await;
        }
        if ffmpeg_missing && let Err(error) = runtime.install_ffmpeg().await {
            let _ = storage
                .append_log("error", "runtime", &error.public_message(), None)
                .await;
        }
        let ready = runtime
            .inspect_all()
            .await
            .iter()
            .all(|component| component.status == RuntimeStatus::Ready);
        status.set_runtime_ready(ready);
    });
}

struct DynamicMediaAnalyzer {
    runtime: RuntimeManager,
}

#[async_trait::async_trait]
impl MediaAnalysis for DynamicMediaAnalyzer {
    async fn analyze(&self, url: &str) -> Result<MediaInfo, FetchError> {
        let path = self.runtime.ytdlp_path().await?;
        let mut adapter = fetch_ytdlp::YtDlp::new(path);
        if let Some(directory) = self.runtime.ffmpeg_directory().await {
            adapter = adapter.with_ffmpeg_directory(directory);
        }
        adapter.inspect_url(url).await
    }
}

fn init_tracing(filter: &str) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new(filter).context("invalid log filter")?)
        .with_target(false)
        .try_init()
        .map_err(|error| anyhow::anyhow!("could not initialize tracing: {error}"))
}

async fn shutdown_signal(shutdown: CancellationToken) {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to listen for shutdown signal");
    }
    info!("shutdown requested");
    shutdown.cancel();
}

#[cfg(test)]
mod listener_tests {
    use super::*;

    #[tokio::test]
    async fn listener_control_hands_off_to_a_new_port() {
        let initial = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let initial_address = initial.local_addr().unwrap();
        let reservation = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let next_address = reservation.local_addr().unwrap();
        drop(reservation);

        let (sender, receiver) = mpsc::channel(1);
        let control = ListenerControl {
            current: Arc::new(RwLock::new(initial_address)),
            sender,
        };
        let app = axum::Router::new().route("/", axum::routing::get(|| async { "ready" }));
        let shutdown = CancellationToken::new();
        let initial_shutdown = CancellationToken::new();
        let tasks = Arc::new(Mutex::new(Vec::new()));
        let (errors, mut error_receiver) = mpsc::unbounded_channel();
        spawn_http_server(
            initial,
            app.clone(),
            initial_shutdown.clone(),
            shutdown.clone(),
            errors.clone(),
            tasks.clone(),
        )
        .await;
        tokio::spawn(rebind_listeners(
            receiver,
            app,
            initial_shutdown,
            shutdown.clone(),
            errors,
            tasks,
        ));

        assert!(control.rebind(next_address).await.unwrap());
        assert!(tokio::net::TcpStream::connect(next_address).await.is_ok());
        tokio::time::sleep(std::time::Duration::from_millis(1_100)).await;
        assert!(
            tokio::net::TcpStream::connect(initial_address)
                .await
                .is_err()
        );
        assert!(error_receiver.try_recv().is_err());
        shutdown.cancel();
    }
}
