mod config;
mod diagnostics;
mod downloads;
mod library;
mod proxy;
mod startup;
pub mod telegram;
mod tray;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use config::FetchConfig;
use diagnostics::DiagnosticsService;
use downloads::{DownloadManager, YtDlpPolicies};
use fetch_core::{
    ApplicationSettings, EventBus, FetchError, ListenerOperations, MediaAnalysis, MediaInfo,
    ProxyPolicy, RuntimeStatus, StatusService,
};
use fetch_runtime::{RuntimeManager, RuntimePaths};
use fetch_storage::Storage;
use library::CompletedLibrary;
use proxy::ManagedProxy;
use startup::{ManagedSettings, SystemStartupRegistration};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const GRACEFUL_SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Clone, Copy)]
struct LaunchOptions {
    background: bool,
    no_tray: bool,
}

fn main() -> anyhow::Result<()> {
    let options = launch_options()?;
    #[cfg(target_os = "windows")]
    configure_windows_console(options);
    let config = FetchConfig::load().context("could not load Fetch configuration")?;
    init_tracing(&config.log_filter)?;

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    if !options.no_tray {
        return tray::run(move |ready| {
            tokio::runtime::Runtime::new()
                .map_err(|error| error.to_string())?
                .block_on(run_fetch(config, options, Some(ready)))
                .map_err(|error| error.to_string())
        });
    }

    tokio::runtime::Runtime::new()?.block_on(run_fetch(config, options, None))
}

async fn run_fetch(
    config: FetchConfig,
    options: LaunchOptions,
    ready: Option<Box<dyn FnOnce(tray::TrayContext) + Send>>,
) -> anyhow::Result<()> {
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
                start_with_system: config.start_with_system,
                ytdlp_auto_update: config.ytdlp_auto_update,
                ytdlp_js_runtime: fetch_core::YtDlpJsRuntime::Auto,
            };
            settings.validate_basic()?;
            storage.save_settings(&settings).await?;
            settings
        }
    };
    let proxy_policy = ProxyPolicy::new(storage.load_proxy_settings().await?)?;
    let js_runtime_policy = fetch_core::JsRuntimePolicy::new(settings.ytdlp_js_runtime);
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
        YtDlpPolicies {
            proxy: proxy_policy.clone(),
            js_runtime: js_runtime_policy.clone(),
        },
    ));
    let media = Arc::new(DynamicMediaAnalyzer {
        runtime: runtime.clone(),
        proxy: proxy_policy.clone(),
        js_runtime: js_runtime_policy,
    });
    let network_policy = fetch_server::NetworkPolicy::new(&settings.allowed_networks)?;
    let shutdown = CancellationToken::new();
    let address = SocketAddr::new(settings.bind_address, settings.port);
    let tray_state = Arc::new(tray::TrayState::new(
        address,
        settings.download_directory.clone(),
    ));
    let startup =
        Arc::new(SystemStartupRegistration::new().map_err(|error| {
            anyhow::anyhow!("could not initialize startup registration: {error}")
        })?);
    let managed_settings = Arc::new(ManagedSettings::new(
        storage.clone(),
        startup,
        tray_state.clone(),
    ));
    if let Err(error) = managed_settings
        .reconcile_startup(settings.start_with_system)
        .await
    {
        warn!(%error, "could not reconcile Start Fetch with system; the server will continue");
    }
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("could not bind HTTP server to {address}"))?;
    let (rebind_sender, rebind_receiver) = mpsc::channel(4);
    let listener_control = Arc::new(ListenerControl {
        current: Arc::new(RwLock::new(address)),
        sender: rebind_sender,
        tray_state: tray_state.clone(),
    });
    let app = fetch_server::router(fetch_server::ServerServices {
        status: status.clone(),
        runtime: runtime.clone(),
        media,
        downloads: downloads.clone(),
        events,
        completed: Arc::new(CompletedLibrary::new(storage.clone())),
        settings: managed_settings,
        proxy: Arc::new(ManagedProxy::new(storage.clone(), proxy_policy)),
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
    if settings.open_browser_on_start && !options.background {
        let url = local_url(address);
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

    let tray_context = tray::TrayContext {
        state: tray_state,
        shutdown: shutdown.clone(),
    };
    if let Some(ready) = ready {
        ready(tray_context);
    } else if !options.no_tray {
        #[cfg(target_os = "linux")]
        tray::start(tray_context).await;
    }

    let unexpected_error = tokio::select! {
        () = shutdown_signal(shutdown.clone()) => None,
        error = server_error_receiver.recv() => error,
    };
    shutdown.cancel();
    let active_tasks = std::mem::take(&mut *tasks.lock().await);
    if tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, async {
        downloads.shutdown().await;
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
    tray_state: Arc<tray::TrayState>,
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
        self.tray_state.set_address(address);
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
        let ytdlp_operation =
            if ytdlp.is_none_or(|component| component.status != RuntimeStatus::Ready) {
                Some("install")
            } else if settings.ytdlp_auto_update && update_due {
                Some("automatic update")
            } else {
                None
            };
        let ytdlp_result = match ytdlp_operation {
            Some(operation) => {
                let details =
                    format!("component: yt-dlp\naction: {operation}\nsource: startup bootstrap");
                let _ = storage
                    .append_log(
                        "info",
                        "runtime",
                        "runtime operation started",
                        Some(&details),
                    )
                    .await;
                info!(
                    component = "yt-dlp",
                    action = operation,
                    "runtime operation started"
                );
                if operation == "install" {
                    runtime.install_ytdlp().await.map(|_| ())
                } else {
                    runtime.update_ytdlp().await.map(|_| ())
                }
            }
            None => Ok(()),
        };
        if let (Some(operation), Err(error)) = (ytdlp_operation, &ytdlp_result) {
            let retained = runtime.components().await.iter().any(|component| {
                component.name == fetch_core::RuntimeComponentName::YtDlp
                    && component.status == RuntimeStatus::Ready
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
            let details = format!("component: yt-dlp\naction: {operation}\n{cause}");
            warn!(
                component = "yt-dlp",
                action = operation,
                retained,
                error = %error.public_message(),
                details = ?error.diagnostic_details(),
                "runtime operation failed"
            );
            let _ = storage
                .append_log(level, "runtime", message, Some(&details))
                .await;
        } else if let Some(operation) = ytdlp_operation {
            let details = format!("component: yt-dlp\naction: {operation}\nresult: completed");
            let _ = storage
                .append_log(
                    "info",
                    "runtime",
                    "runtime operation completed",
                    Some(&details),
                )
                .await;
            info!(
                component = "yt-dlp",
                action = operation,
                "runtime operation completed"
            );
        }
        if ytdlp_operation.is_some() && ytdlp_result.is_ok() {
            let _ = storage
                .set_metadata("ytdlp_last_update", &chrono::Utc::now().to_rfc3339())
                .await;
        }
        if ffmpeg_missing {
            let start_details =
                "component: ffmpeg/ffprobe\naction: install\nsource: startup bootstrap";
            let _ = storage
                .append_log(
                    "info",
                    "runtime",
                    "runtime operation started",
                    Some(start_details),
                )
                .await;
            info!(
                component = "ffmpeg/ffprobe",
                action = "install",
                "runtime operation started"
            );
            match runtime.install_ffmpeg().await {
                Ok(_) => {
                    let _ = storage
                        .append_log(
                            "info",
                            "runtime",
                            "runtime operation completed",
                            Some("component: ffmpeg/ffprobe\naction: install\nresult: completed"),
                        )
                        .await;
                    info!(
                        component = "ffmpeg/ffprobe",
                        action = "install",
                        "runtime operation completed"
                    );
                }
                Err(error) => {
                    let cause = error
                        .diagnostic_details()
                        .map(str::to_owned)
                        .unwrap_or_else(|| error.public_message());
                    let details = format!("component: ffmpeg/ffprobe\naction: install\n{cause}");
                    warn!(
                        component = "ffmpeg/ffprobe",
                        action = "install",
                        error = %error.public_message(),
                        details = ?error.diagnostic_details(),
                        "runtime operation failed"
                    );
                    let _ = storage
                        .append_log(
                            "error",
                            "runtime",
                            "runtime operation failed",
                            Some(&details),
                        )
                        .await;
                }
            }
        }
        let ready = runtime
            .components()
            .await
            .iter()
            .all(|component| component.status == RuntimeStatus::Ready);
        status.set_runtime_ready(ready);
    });
}

struct DynamicMediaAnalyzer {
    runtime: RuntimeManager,
    proxy: ProxyPolicy,
    js_runtime: fetch_core::JsRuntimePolicy,
}

#[async_trait::async_trait]
impl MediaAnalysis for DynamicMediaAnalyzer {
    async fn analyze(&self, url: &str) -> Result<MediaInfo, FetchError> {
        let path = self.runtime.ytdlp_path().await?;
        let mut adapter = fetch_ytdlp::YtDlp::new(path)
            .with_proxy(self.proxy.current())?
            .with_js_runtime(self.js_runtime.current());
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

fn local_url(address: SocketAddr) -> String {
    let ip = if address.ip().is_unspecified() {
        std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
    } else {
        address.ip()
    };
    format!("http://{}/", SocketAddr::new(ip, address.port()))
}

fn launch_options() -> anyhow::Result<LaunchOptions> {
    let mut options = LaunchOptions {
        background: false,
        no_tray: false,
    };
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--background" => options.background = true,
            "--no-tray" => options.no_tray = true,
            _ => return Err(anyhow::anyhow!("unknown argument: {argument}")),
        }
    }
    Ok(options)
}

#[cfg(target_os = "windows")]
fn configure_windows_console(options: LaunchOptions) {
    use windows_sys::Win32::{
        System::Console::{GetConsoleProcessList, GetConsoleWindow},
        UI::WindowsAndMessaging::{SW_HIDE, ShowWindow},
    };

    let mut processes = [0_u32; 2];
    // A console containing only Fetch was allocated by direct desktop launch.
    // Preserve shared terminal consoles so Ctrl+C and logs remain available.
    let owns_console = unsafe { GetConsoleProcessList(processes.as_mut_ptr(), 2) } <= 1;
    if options.background || (!options.no_tray && owns_console) {
        let window = unsafe { GetConsoleWindow() };
        if !window.is_null() {
            unsafe {
                ShowWindow(window, SW_HIDE);
            }
        }
    }
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
            tray_state: Arc::new(tray::TrayState::new(initial_address, "downloads".into())),
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
