//! Managed external runtime discovery, health checks, and safe installation.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use fetch_core::{
    FetchError, JavaScriptRuntime, JavaScriptRuntimeName, RuntimeComponent, RuntimeComponentName,
    RuntimeStatus,
};
use sha2::{Digest, Sha256};
use tokio::{
    io::AsyncWriteExt,
    process::Command,
    sync::{Mutex, RwLock, broadcast},
};
use tracing::{info, warn};

const YTDLP_CHECKSUMS_URL: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
const FFMPEG_RELEASE_API: &str =
    "https://api.github.com/repos/eugeneware/ffmpeg-static/releases/latest";
const RUNTIME_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const RUNTIME_READ_TIMEOUT: Duration = Duration::from_secs(30);
const RUNTIME_REQUEST_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
pub struct RuntimePaths {
    pub root: PathBuf,
}

impl RuntimePaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn ytdlp_directory(&self) -> PathBuf {
        self.root.join("yt-dlp")
    }

    pub fn ffmpeg_directory(&self) -> PathBuf {
        self.root.join("ffmpeg")
    }

    pub fn ytdlp_executable(&self) -> PathBuf {
        self.ytdlp_directory().join(executable_filename("yt-dlp"))
    }

    pub fn ffmpeg_executable(&self) -> PathBuf {
        self.ffmpeg_directory().join(executable_filename("ffmpeg"))
    }

    pub fn ffprobe_executable(&self) -> PathBuf {
        self.ffmpeg_directory().join(executable_filename("ffprobe"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAsset {
    pub component: RuntimeComponentName,
    pub platform: &'static str,
    pub architecture: &'static str,
    pub filename: &'static str,
    pub url: String,
    pub checksum_url: String,
    pub expected_sha256: Option<String>,
}

pub trait RuntimeSource: Send + Sync {
    fn ytdlp_asset(&self) -> Result<RuntimeAsset, FetchError>;
}

#[derive(Debug, Default)]
pub struct OfficialYtDlpSource;

impl RuntimeSource for OfficialYtDlpSource {
    fn ytdlp_asset(&self) -> Result<RuntimeAsset, FetchError> {
        ytdlp_asset_for(std::env::consts::OS, std::env::consts::ARCH)
    }
}

fn ytdlp_asset_for(
    platform: &'static str,
    architecture: &'static str,
) -> Result<RuntimeAsset, FetchError> {
    let filename = match (platform, architecture) {
        ("linux", "x86_64") => "yt-dlp_linux",
        ("linux", "aarch64") => "yt-dlp_linux_aarch64",
        ("macos", "x86_64" | "aarch64") => "yt-dlp_macos",
        ("windows", "x86_64") => "yt-dlp.exe",
        _ => {
            return Err(FetchError::RuntimeMissing(format!(
                "yt-dlp has no managed asset for {platform}/{architecture}"
            )));
        }
    };
    Ok(RuntimeAsset {
        component: RuntimeComponentName::YtDlp,
        platform,
        architecture,
        filename,
        url: format!("https://github.com/yt-dlp/yt-dlp/releases/latest/download/{filename}"),
        checksum_url: YTDLP_CHECKSUMS_URL.into(),
        expected_sha256: None,
    })
}

#[derive(Clone)]
pub struct RuntimeManager {
    paths: RuntimePaths,
    source: Arc<dyn RuntimeSource>,
    client: reqwest::Client,
    state: Arc<RwLock<HashMap<RuntimeComponentName, RuntimeComponent>>>,
    events: broadcast::Sender<RuntimeComponent>,
    operation_lock: Arc<Mutex<()>>,
}

impl RuntimeManager {
    pub fn new(paths: RuntimePaths) -> Result<Self, FetchError> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .user_agent(concat!("Fetch/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(RUNTIME_CONNECT_TIMEOUT)
            .read_timeout(RUNTIME_READ_TIMEOUT)
            .timeout(RUNTIME_REQUEST_TIMEOUT)
            .build()
            .map_err(|error| FetchError::Internal(error.to_string()))?;
        Ok(Self::with_source(
            paths,
            Arc::new(OfficialYtDlpSource),
            client,
        ))
    }

    pub fn with_source(
        paths: RuntimePaths,
        source: Arc<dyn RuntimeSource>,
        client: reqwest::Client,
    ) -> Self {
        let (events, _) = broadcast::channel(64);
        Self {
            paths,
            source,
            client,
            state: Arc::new(RwLock::new(HashMap::new())),
            events,
            operation_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn paths(&self) -> &RuntimePaths {
        &self.paths
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeComponent> {
        self.events.subscribe()
    }

    pub async fn inspect_all(&self) -> Vec<RuntimeComponent> {
        let ytdlp = self.inspect_component(RuntimeComponentName::YtDlp).await;
        let ffmpeg = self.inspect_component(RuntimeComponentName::Ffmpeg).await;
        let ffprobe = self.inspect_component(RuntimeComponentName::Ffprobe).await;
        let components = vec![ytdlp, ffmpeg, ffprobe];
        let mut state = self.state.write().await;
        for component in &components {
            state.insert(component.name, component.clone());
        }
        components
    }

    pub async fn components(&self) -> Vec<RuntimeComponent> {
        let state = self.state.read().await;
        if state.is_empty() {
            drop(state);
            return self.inspect_all().await;
        }
        [
            RuntimeComponentName::YtDlp,
            RuntimeComponentName::Ffmpeg,
            RuntimeComponentName::Ffprobe,
        ]
        .into_iter()
        .filter_map(|name| state.get(&name).cloned())
        .collect()
    }

    pub async fn javascript_runtimes(&self) -> Vec<JavaScriptRuntime> {
        let mut runtimes = Vec::with_capacity(3);
        for name in [
            JavaScriptRuntimeName::Deno,
            JavaScriptRuntimeName::Node,
            JavaScriptRuntimeName::QuickJs,
        ] {
            let executable = which::which(name.executable_name()).ok();
            runtimes.push(inspect_javascript_runtime(name, executable.as_deref()).await);
        }
        runtimes
    }

    pub async fn ytdlp_path(&self) -> Result<PathBuf, FetchError> {
        locate_executable(&self.paths.ytdlp_executable(), "yt-dlp")
            .ok_or_else(|| FetchError::RuntimeMissing("yt-dlp".into()))
    }

    pub async fn ffmpeg_directory(&self) -> Option<PathBuf> {
        if self.paths.ffmpeg_executable().is_file() && self.paths.ffprobe_executable().is_file() {
            return Some(self.paths.ffmpeg_directory());
        }
        which::which("ffmpeg")
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
    }

    pub async fn install_ytdlp(&self) -> Result<RuntimeComponent, FetchError> {
        let _operation = self.operation_lock.lock().await;
        self.set_state(
            RuntimeComponentName::YtDlp,
            RuntimeStatus::Installing,
            None,
            None,
        )
        .await;
        let result = self.install_ytdlp_inner().await;
        match result {
            Ok(component) => {
                self.store_component(component.clone()).await;
                Ok(component)
            }
            Err(error) => {
                self.set_state(
                    RuntimeComponentName::YtDlp,
                    RuntimeStatus::Failed,
                    None,
                    Some(error.public_message()),
                )
                .await;
                Err(error)
            }
        }
    }

    pub async fn update_ytdlp(&self) -> Result<RuntimeComponent, FetchError> {
        let _operation = self.operation_lock.lock().await;
        let previous = self.inspect_component(RuntimeComponentName::YtDlp).await;
        self.set_state(
            RuntimeComponentName::YtDlp,
            RuntimeStatus::Updating,
            None,
            None,
        )
        .await;
        match self.install_ytdlp_inner().await {
            Ok(component) => {
                self.store_component(component.clone()).await;
                Ok(component)
            }
            Err(error) => {
                if previous.status == RuntimeStatus::Ready {
                    warn!(
                        component = "yt-dlp",
                        operation = "update",
                        error = %error.public_message(),
                        details = ?error.diagnostic_details(),
                        "runtime update failed; keeping the existing verified runtime"
                    );
                    self.store_component(previous).await;
                } else {
                    self.set_state(
                        RuntimeComponentName::YtDlp,
                        RuntimeStatus::Failed,
                        None,
                        Some(error.public_message()),
                    )
                    .await;
                }
                Err(error)
            }
        }
    }

    pub async fn repair_ytdlp(&self) -> Result<RuntimeComponent, FetchError> {
        self.install_ytdlp().await
    }

    pub async fn install_ffmpeg(&self) -> Result<Vec<RuntimeComponent>, FetchError> {
        let _operation = self.operation_lock.lock().await;
        self.set_ffmpeg_state(RuntimeStatus::Installing, None, None)
            .await;
        self.install_ffmpeg_inner(RuntimeStatus::Installing).await
    }

    pub async fn update_ffmpeg(&self) -> Result<Vec<RuntimeComponent>, FetchError> {
        let _operation = self.operation_lock.lock().await;
        let previous = vec![
            self.inspect_component(RuntimeComponentName::Ffmpeg).await,
            self.inspect_component(RuntimeComponentName::Ffprobe).await,
        ];
        self.set_ffmpeg_state(RuntimeStatus::Updating, None, None)
            .await;
        match self.install_ffmpeg_artifacts(RuntimeStatus::Updating).await {
            Ok(components) => {
                for component in &components {
                    self.store_component(component.clone()).await;
                }
                Ok(components)
            }
            Err(error) => {
                if previous
                    .iter()
                    .all(|component| component.status == RuntimeStatus::Ready)
                {
                    warn!(
                        component = "ffmpeg/ffprobe",
                        operation = "update",
                        error = %error.public_message(),
                        details = ?error.diagnostic_details(),
                        "runtime update failed; keeping the existing verified runtimes"
                    );
                    for component in previous {
                        self.store_component(component).await;
                    }
                } else {
                    self.set_ffmpeg_state(
                        RuntimeStatus::Failed,
                        None,
                        Some(error.public_message()),
                    )
                    .await;
                }
                Err(error)
            }
        }
    }

    pub async fn repair_ffmpeg(&self) -> Result<Vec<RuntimeComponent>, FetchError> {
        self.install_ffmpeg().await
    }

    async fn install_ytdlp_inner(&self) -> Result<RuntimeComponent, FetchError> {
        let asset = self.source.ytdlp_asset()?;
        tokio::fs::create_dir_all(self.paths.ytdlp_directory())
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
        let artifact = self.download(&asset.url).await?;
        self.set_state(
            RuntimeComponentName::YtDlp,
            RuntimeStatus::Installing,
            Some(70.0),
            None,
        )
        .await;
        let checksums = String::from_utf8(self.download(&asset.checksum_url).await?)
            .map_err(|error| FetchError::RuntimeCorrupt(error.to_string()))?;
        verify_checksum(&artifact, &checksums, asset.filename)?;

        let target = self.paths.ytdlp_executable();
        let temporary = target.with_extension(format!("download-{}", uuid::Uuid::new_v4()));
        let backup = target.with_extension("previous");
        write_executable(&temporary, &artifact).await?;
        let version = executable_version(&temporary).await?;
        self.set_state(
            RuntimeComponentName::YtDlp,
            RuntimeStatus::Installing,
            Some(90.0),
            None,
        )
        .await;

        if target.exists() {
            if backup.exists() {
                tokio::fs::remove_file(&backup)
                    .await
                    .map_err(|error| FetchError::Internal(error.to_string()))?;
            }
            tokio::fs::rename(&target, &backup)
                .await
                .map_err(|error| FetchError::Internal(error.to_string()))?;
        }
        if let Err(error) = tokio::fs::rename(&temporary, &target).await {
            if backup.exists() {
                let _ = tokio::fs::rename(&backup, &target).await;
            }
            return Err(FetchError::Internal(format!(
                "atomic runtime replacement failed: {error}"
            )));
        }
        if let Err(error) = executable_version(&target).await {
            let _ = tokio::fs::remove_file(&target).await;
            if backup.exists() {
                let _ = tokio::fs::rename(&backup, &target).await;
            }
            return Err(error);
        }
        if backup.exists()
            && let Err(error) = tokio::fs::remove_file(&backup).await
        {
            warn!(%error, path = %backup.display(), "could not remove old runtime backup");
        }
        info!(%version, path = %target.display(), "yt-dlp runtime installed");
        Ok(RuntimeComponent {
            name: RuntimeComponentName::YtDlp,
            version: Some(version),
            status: RuntimeStatus::Ready,
            progress_percent: Some(100.0),
            error: None,
        })
    }

    async fn install_ffmpeg_inner(
        &self,
        operation: RuntimeStatus,
    ) -> Result<Vec<RuntimeComponent>, FetchError> {
        let result = self.install_ffmpeg_artifacts(operation).await;
        match result {
            Ok(components) => {
                for component in &components {
                    self.store_component(component.clone()).await;
                }
                Ok(components)
            }
            Err(error) => {
                self.set_ffmpeg_state(RuntimeStatus::Failed, None, Some(error.public_message()))
                    .await;
                Err(error)
            }
        }
    }

    async fn install_ffmpeg_artifacts(
        &self,
        operation: RuntimeStatus,
    ) -> Result<Vec<RuntimeComponent>, FetchError> {
        let release: GitHubRelease = self
            .client
            .get(FFMPEG_RELEASE_API)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(runtime_download_error)?
            .error_for_status()
            .map_err(runtime_download_error)?
            .json()
            .await
            .map_err(runtime_download_error)?;
        let platform = ffmpeg_platform_name()?;
        let ffmpeg_name = format!("ffmpeg-{platform}");
        let ffprobe_name = format!("ffprobe-{platform}");
        let license_name = format!("{platform}.LICENSE");
        let ffmpeg_asset = release.asset(&ffmpeg_name)?;
        let ffprobe_asset = release.asset(&ffprobe_name)?;
        let license_asset = release.asset(&license_name)?;
        let (ffmpeg_bytes, ffprobe_bytes, license_bytes) = tokio::try_join!(
            self.download_verified_asset(ffmpeg_asset),
            self.download_verified_asset(ffprobe_asset),
            self.download_verified_asset(license_asset),
        )?;
        self.set_ffmpeg_state(operation, Some(70.0), None).await;

        let directory = self.paths.ffmpeg_directory();
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
        let ffmpeg_target = self.paths.ffmpeg_executable();
        let ffprobe_target = self.paths.ffprobe_executable();
        let suffix = uuid::Uuid::new_v4();
        let ffmpeg_temp = directory.join(format!("ffmpeg-{suffix}.download"));
        let ffprobe_temp = directory.join(format!("ffprobe-{suffix}.download"));
        write_executable(&ffmpeg_temp, &ffmpeg_bytes).await?;
        write_executable(&ffprobe_temp, &ffprobe_bytes).await?;
        let ffmpeg_version = executable_version(&ffmpeg_temp).await?;
        let ffprobe_version = executable_version(&ffprobe_temp).await?;
        let ffmpeg_backup = ffmpeg_target.with_extension("previous");
        let ffprobe_backup = ffprobe_target.with_extension("previous");

        replace_pair_atomically(
            &ffmpeg_temp,
            &ffmpeg_target,
            &ffmpeg_backup,
            &ffprobe_temp,
            &ffprobe_target,
            &ffprobe_backup,
        )
        .await?;
        if let Err(error) = executable_version(&ffmpeg_target).await {
            rollback_pair(
                &ffmpeg_target,
                &ffmpeg_backup,
                &ffprobe_target,
                &ffprobe_backup,
            )
            .await;
            return Err(error);
        }
        if let Err(error) = executable_version(&ffprobe_target).await {
            rollback_pair(
                &ffmpeg_target,
                &ffmpeg_backup,
                &ffprobe_target,
                &ffprobe_backup,
            )
            .await;
            return Err(error);
        }
        let _ = tokio::fs::remove_file(&ffmpeg_backup).await;
        let _ = tokio::fs::remove_file(&ffprobe_backup).await;
        tokio::fs::write(directory.join("FFMPEG-LICENSE.txt"), license_bytes)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
        tokio::fs::write(directory.join("PROVIDER.txt"), format!(
            "eugeneware/ffmpeg-static {}\nhttps://github.com/eugeneware/ffmpeg-static\nGPL-3.0-or-later\n",
            release.tag_name
        )).await.map_err(|error| FetchError::Internal(error.to_string()))?;
        Ok(vec![
            RuntimeComponent {
                name: RuntimeComponentName::Ffmpeg,
                version: Some(ffmpeg_version),
                status: RuntimeStatus::Ready,
                progress_percent: Some(100.0),
                error: None,
            },
            RuntimeComponent {
                name: RuntimeComponentName::Ffprobe,
                version: Some(ffprobe_version),
                status: RuntimeStatus::Ready,
                progress_percent: Some(100.0),
                error: None,
            },
        ])
    }

    async fn download_verified_asset(&self, asset: &GitHubAsset) -> Result<Vec<u8>, FetchError> {
        let bytes = self.download(&asset.browser_download_url).await?;
        let expected = asset
            .digest
            .as_deref()
            .and_then(|digest| digest.strip_prefix("sha256:"))
            .ok_or_else(|| {
                FetchError::RuntimeCorrupt(format!(
                    "provider did not publish a SHA-256 digest for {}",
                    asset.name
                ))
            })?;
        verify_expected_checksum(&bytes, expected, &asset.name)?;
        Ok(bytes)
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| runtime_request_error("send request", url, error))?
            .error_for_status()
            .map_err(|error| runtime_request_error("check response status", url, error))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|error| runtime_request_error("read response body", url, error))?;
        Ok(bytes.to_vec())
    }

    async fn inspect_component(&self, name: RuntimeComponentName) -> RuntimeComponent {
        let (managed, command) = match name {
            RuntimeComponentName::YtDlp => (self.paths.ytdlp_executable(), "yt-dlp"),
            RuntimeComponentName::Ffmpeg => (self.paths.ffmpeg_executable(), "ffmpeg"),
            RuntimeComponentName::Ffprobe => (self.paths.ffprobe_executable(), "ffprobe"),
        };
        let Some(path) = locate_executable(&managed, command) else {
            return RuntimeComponent {
                name,
                version: None,
                status: RuntimeStatus::Missing,
                progress_percent: None,
                error: None,
            };
        };
        match executable_version(&path).await {
            Ok(version) => RuntimeComponent {
                name,
                version: Some(version),
                status: RuntimeStatus::Ready,
                progress_percent: None,
                error: None,
            },
            Err(error) => RuntimeComponent {
                name,
                version: None,
                status: RuntimeStatus::Failed,
                progress_percent: None,
                error: Some(error.public_message()),
            },
        }
    }

    async fn set_state(
        &self,
        name: RuntimeComponentName,
        status: RuntimeStatus,
        progress_percent: Option<f64>,
        error: Option<String>,
    ) {
        self.store_component(RuntimeComponent {
            name,
            version: None,
            status,
            progress_percent,
            error,
        })
        .await;
    }

    async fn set_ffmpeg_state(
        &self,
        status: RuntimeStatus,
        progress_percent: Option<f64>,
        error: Option<String>,
    ) {
        for name in [RuntimeComponentName::Ffmpeg, RuntimeComponentName::Ffprobe] {
            self.set_state(name, status, progress_percent, error.clone())
                .await;
        }
    }

    async fn store_component(&self, component: RuntimeComponent) {
        self.state
            .write()
            .await
            .insert(component.name, component.clone());
        let _ = self.events.send(component);
    }
}

async fn inspect_javascript_runtime(
    name: JavaScriptRuntimeName,
    executable: Option<&Path>,
) -> JavaScriptRuntime {
    let Some(executable) = executable else {
        return JavaScriptRuntime {
            name,
            detected: false,
            version: None,
        };
    };
    match executable_version(executable).await {
        Ok(version) => JavaScriptRuntime {
            name,
            detected: true,
            version: Some(version),
        },
        Err(error) => {
            warn!(
                runtime = ?name,
                executable = %executable.display(),
                error = %error.public_message(),
                "JavaScript runtime was found but its version could not be read"
            );
            JavaScriptRuntime {
                name,
                detected: true,
                version: None,
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

impl GitHubRelease {
    fn asset(&self, name: &str) -> Result<&GitHubAsset, FetchError> {
        self.assets
            .iter()
            .find(|asset| asset.name == name)
            .ok_or_else(|| FetchError::RuntimeMissing(format!("release asset {name} is missing")))
    }
}

#[derive(Debug, serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

fn ffmpeg_platform_name() -> Result<&'static str, FetchError> {
    ffmpeg_platform_for(std::env::consts::OS, std::env::consts::ARCH)
}

fn ffmpeg_platform_for(platform: &str, architecture: &str) -> Result<&'static str, FetchError> {
    match (platform, architecture) {
        ("linux", "x86_64") => Ok("linux-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("macos", "x86_64") => Ok("darwin-x64"),
        ("macos", "aarch64") => Ok("darwin-arm64"),
        ("windows", "x86_64") => Ok("win32-x64"),
        (platform, architecture) => Err(FetchError::RuntimeMissing(format!(
            "FFmpeg has no managed asset for {platform}/{architecture}"
        ))),
    }
}

fn runtime_download_error(error: reqwest::Error) -> FetchError {
    runtime_request_error("load provider release metadata", FFMPEG_RELEASE_API, error)
}

fn runtime_request_error(stage: &str, url: &str, error: reqwest::Error) -> FetchError {
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connection"
    } else if error.is_status() {
        "HTTP status"
    } else if error.is_body() {
        "response body"
    } else {
        "request"
    };
    FetchError::ProcessFailed {
        summary: format!("runtime provider {kind} failure"),
        details: format!("stage: {stage}\nprovider: {url}\nkind: {kind}\ncause: {error}"),
    }
}

fn locate_executable(managed: &Path, system_name: &str) -> Option<PathBuf> {
    managed
        .is_file()
        .then(|| managed.to_path_buf())
        .or_else(|| which::which(system_name).ok())
}

async fn executable_version(path: &Path) -> Result<String, FetchError> {
    let mut output = Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| FetchError::RuntimeCorrupt(format!("{}: {error}", path.display())))?;
    if !output.status.success() {
        output = Command::new(path)
            .arg("-version")
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|error| FetchError::RuntimeCorrupt(format!("{}: {error}", path.display())))?;
        if !output.status.success() {
            return Err(FetchError::RuntimeCorrupt(path.display().to_string()));
        }
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.lines().next().unwrap_or_default().trim().to_owned();
    if version.is_empty() {
        Err(FetchError::RuntimeCorrupt(path.display().to_string()))
    } else {
        Ok(version)
    }
}

fn verify_checksum(
    artifact: &[u8],
    checksum_document: &str,
    filename: &str,
) -> Result<(), FetchError> {
    let expected = checksum_document
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let checksum = fields.next()?;
            let name = fields.next()?.trim_start_matches('*');
            (name == filename).then_some(checksum)
        })
        .ok_or_else(|| FetchError::RuntimeCorrupt(format!("checksum for {filename} is missing")))?;
    let actual = format!("{:x}", Sha256::digest(artifact));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(FetchError::RuntimeCorrupt(format!(
            "checksum mismatch for {filename}"
        )))
    }
}

fn verify_expected_checksum(
    artifact: &[u8],
    expected: &str,
    filename: &str,
) -> Result<(), FetchError> {
    let actual = format!("{:x}", Sha256::digest(artifact));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(FetchError::RuntimeCorrupt(format!(
            "checksum mismatch for {filename}"
        )))
    }
}

async fn replace_pair_atomically(
    first_temp: &Path,
    first_target: &Path,
    first_backup: &Path,
    second_temp: &Path,
    second_target: &Path,
    second_backup: &Path,
) -> Result<(), FetchError> {
    backup_existing(first_target, first_backup).await?;
    if let Err(error) = backup_existing(second_target, second_backup).await {
        restore_backup(first_target, first_backup).await;
        return Err(error);
    }
    if let Err(error) = tokio::fs::rename(first_temp, first_target).await {
        restore_backup(first_target, first_backup).await;
        restore_backup(second_target, second_backup).await;
        return Err(FetchError::Internal(format!(
            "FFmpeg atomic replacement failed: {error}"
        )));
    }
    if let Err(error) = tokio::fs::rename(second_temp, second_target).await {
        let _ = tokio::fs::remove_file(first_target).await;
        restore_backup(first_target, first_backup).await;
        restore_backup(second_target, second_backup).await;
        return Err(FetchError::Internal(format!(
            "FFprobe atomic replacement failed: {error}"
        )));
    }
    Ok(())
}

async fn backup_existing(target: &Path, backup: &Path) -> Result<(), FetchError> {
    if backup.exists() {
        tokio::fs::remove_file(backup)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
    }
    if target.exists() {
        tokio::fs::rename(target, backup)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
    }
    Ok(())
}

async fn restore_backup(target: &Path, backup: &Path) {
    if backup.exists() {
        let _ = tokio::fs::remove_file(target).await;
        let _ = tokio::fs::rename(backup, target).await;
    }
}

async fn rollback_pair(
    first_target: &Path,
    first_backup: &Path,
    second_target: &Path,
    second_backup: &Path,
) {
    restore_backup(first_target, first_backup).await;
    restore_backup(second_target, second_backup).await;
}

async fn write_executable(path: &Path, bytes: &[u8]) -> Result<(), FetchError> {
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|error| FetchError::Internal(error.to_string()))?;
    file.write_all(bytes)
        .await
        .map_err(|error| FetchError::Internal(error.to_string()))?;
    file.flush()
        .await
        .map_err(|error| FetchError::Internal(error.to_string()))?;
    file.sync_all()
        .await
        .map_err(|error| FetchError::Internal(error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = file
            .metadata()
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?
            .permissions();
        permissions.set_mode(0o755);
        tokio::fs::set_permissions(path, permissions)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
    }
    Ok(())
}

fn executable_filename(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UnreachableYtDlpSource {
        url: String,
    }

    impl RuntimeSource for UnreachableYtDlpSource {
        fn ytdlp_asset(&self) -> Result<RuntimeAsset, FetchError> {
            Ok(RuntimeAsset {
                component: RuntimeComponentName::YtDlp,
                platform: "test",
                architecture: "test",
                filename: "yt-dlp",
                url: self.url.clone(),
                checksum_url: self.url.clone(),
                expected_sha256: None,
            })
        }
    }

    #[test]
    fn checksum_validation_accepts_matching_and_rejects_corruption() {
        let bytes = b"runtime";
        let digest = format!("{:x}", Sha256::digest(bytes));
        let document = format!("{digest}  yt-dlp_linux\n");
        assert!(verify_checksum(bytes, &document, "yt-dlp_linux").is_ok());
        assert!(verify_checksum(b"corrupt", &document, "yt-dlp_linux").is_err());
    }

    #[test]
    fn paths_keep_components_under_runtime_root() {
        let paths = RuntimePaths::new("/data/runtime");
        assert!(paths.ytdlp_executable().starts_with("/data/runtime/yt-dlp"));
        assert!(
            paths
                .ffmpeg_executable()
                .starts_with("/data/runtime/ffmpeg")
        );
    }

    #[test]
    fn manifests_cover_every_release_target() {
        for (platform, architecture) in [
            ("linux", "x86_64"),
            ("linux", "aarch64"),
            ("macos", "x86_64"),
            ("macos", "aarch64"),
            ("windows", "x86_64"),
        ] {
            assert!(ytdlp_asset_for(platform, architecture).is_ok());
            assert!(ffmpeg_platform_for(platform, architecture).is_ok());
        }
        assert!(ytdlp_asset_for("plan9", "mips").is_err());
        assert!(ffmpeg_platform_for("plan9", "mips").is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn javascript_runtime_inspection_reports_detected_versions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("node");
        tokio::fs::write(&executable, b"#!/bin/sh\nprintf 'v25.9.0\\n'\n")
            .await
            .unwrap();
        let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions).unwrap();

        let runtime =
            inspect_javascript_runtime(JavaScriptRuntimeName::Node, Some(&executable)).await;
        assert!(runtime.detected);
        assert_eq!(runtime.version.as_deref(), Some("v25.9.0"));

        let missing = inspect_javascript_runtime(JavaScriptRuntimeName::Deno, None).await;
        assert!(!missing.detected);
        assert_eq!(missing.version, None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_update_keeps_an_existing_healthy_ytdlp_ready() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let paths = RuntimePaths::new(directory.path());
        tokio::fs::create_dir_all(paths.ytdlp_directory())
            .await
            .unwrap();
        tokio::fs::write(
            paths.ytdlp_executable(),
            b"#!/bin/sh\nprintf 'working-version\\n'\n",
        )
        .await
        .unwrap();
        let mut permissions = std::fs::metadata(paths.ytdlp_executable())
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(paths.ytdlp_executable(), permissions).unwrap();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let unreachable = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_millis(100))
            .timeout(Duration::from_millis(250))
            .build()
            .unwrap();
        let manager = RuntimeManager::with_source(
            paths,
            Arc::new(UnreachableYtDlpSource { url: unreachable }),
            client,
        );

        assert!(manager.update_ytdlp().await.is_err());
        let component = manager
            .components()
            .await
            .into_iter()
            .find(|component| component.name == RuntimeComponentName::YtDlp)
            .unwrap();
        assert_eq!(component.status, RuntimeStatus::Ready);
        assert_eq!(component.version.as_deref(), Some("working-version"));
        assert_eq!(
            manager.ytdlp_path().await.unwrap(),
            manager.paths().ytdlp_executable()
        );
    }

    #[tokio::test]
    async fn failed_first_install_retains_the_actionable_failed_state() {
        let directory = tempfile::tempdir().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let unreachable = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_millis(100))
            .timeout(Duration::from_millis(250))
            .build()
            .unwrap();
        let manager = RuntimeManager::with_source(
            RuntimePaths::new(directory.path()),
            Arc::new(UnreachableYtDlpSource { url: unreachable }),
            client,
        );

        assert!(manager.install_ytdlp().await.is_err());
        let component = manager
            .components()
            .await
            .into_iter()
            .find(|component| component.name == RuntimeComponentName::YtDlp)
            .unwrap();
        assert_eq!(component.status, RuntimeStatus::Failed);
        assert_eq!(
            component.error.as_deref(),
            Some("runtime provider connection failure")
        );
    }

    #[test]
    fn runtime_request_errors_include_actionable_context() {
        let error = reqwest::Client::builder()
            .build()
            .unwrap()
            .get("not a URL")
            .build()
            .unwrap_err();
        let mapped =
            runtime_request_error("load test artifact", "https://provider.test/tool", error);
        assert_eq!(mapped.public_message(), "runtime provider request failure");
        let details = mapped.diagnostic_details().unwrap();
        assert!(details.contains("stage: load test artifact"));
        assert!(details.contains("provider: https://provider.test/tool"));
        assert!(details.contains("kind: request"));
        assert!(details.contains("cause:"));
    }

    #[tokio::test]
    async fn failed_pair_replacement_restores_both_working_binaries() {
        let directory = tempfile::tempdir().unwrap();
        let first_target = directory.path().join("ffmpeg");
        let second_target = directory.path().join("ffprobe");
        let first_temp = directory.path().join("ffmpeg.download");
        let missing_second_temp = directory.path().join("ffprobe.download");
        let first_backup = directory.path().join("ffmpeg.previous");
        let second_backup = directory.path().join("ffprobe.previous");
        tokio::fs::write(&first_target, b"working ffmpeg")
            .await
            .unwrap();
        tokio::fs::write(&second_target, b"working ffprobe")
            .await
            .unwrap();
        tokio::fs::write(&first_temp, b"new ffmpeg").await.unwrap();

        assert!(
            replace_pair_atomically(
                &first_temp,
                &first_target,
                &first_backup,
                &missing_second_temp,
                &second_target,
                &second_backup,
            )
            .await
            .is_err()
        );
        assert_eq!(
            tokio::fs::read(&first_target).await.unwrap(),
            b"working ffmpeg"
        );
        assert_eq!(
            tokio::fs::read(&second_target).await.unwrap(),
            b"working ffprobe"
        );
    }
}
