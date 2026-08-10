mod config;
mod diagnostics;
mod downloads;
mod library;

use std::{future::IntoFuture, sync::Arc};

use anyhow::Context;
use config::FetchConfig;
use diagnostics::DiagnosticsService;
use downloads::DownloadManager;
use fetch_core::{
    ApplicationSettings, EventBus, FetchError, MediaAnalysis, MediaInfo, RuntimeStatus,
    StatusService,
};
use fetch_runtime::{RuntimeManager, RuntimePaths};
use fetch_storage::Storage;
use library::CompletedLibrary;
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
    let app = fetch_server::router(fetch_server::ServerServices {
        status: status.clone(),
        runtime: runtime.clone(),
        media,
        downloads,
        events,
        completed: Arc::new(CompletedLibrary::new(storage.clone())),
        settings: storage.clone(),
        network_policy,
        diagnostics: Arc::new(DiagnosticsService::new(storage.clone(), runtime.clone())),
        shutdown: shutdown.clone(),
    });
    start_runtime_bootstrap(runtime, status, storage, settings.clone());
    let address = std::net::SocketAddr::new(settings.bind_address, settings.port);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("could not bind HTTP server to {address}"))?;

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

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown.clone()))
    .into_future();
    tokio::pin!(server);
    let force_shutdown = async {
        shutdown.cancelled().await;
        tokio::time::sleep(GRACEFUL_SHUTDOWN_TIMEOUT).await;
    };
    tokio::pin!(force_shutdown);
    tokio::select! {
        result = &mut server => result.context("HTTP server stopped unexpectedly")?,
        () = &mut force_shutdown => {
            warn!(timeout_seconds = GRACEFUL_SHUTDOWN_TIMEOUT.as_secs(), "graceful shutdown timed out; closing remaining connections");
        }
    }
    Ok(())
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
