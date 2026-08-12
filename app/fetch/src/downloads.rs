use std::{
    collections::HashMap,
    path::PathBuf,
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use chrono::Utc;
use fetch_core::{
    ApplicationEvent, CompletedFile, DownloadJob, DownloadOperations, DownloadRequest,
    DownloadStatus, EventBus, FetchError,
};
use fetch_runtime::RuntimeManager;
use fetch_storage::Storage;
use fetch_ytdlp::{DownloadArtifacts, ProcessLine, YtDlp, map_process_error, parse_process_line};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{Mutex, Notify, RwLock, mpsc},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct DownloadManager {
    inner: Arc<Inner>,
}

struct Inner {
    storage: Arc<Storage>,
    runtime: RuntimeManager,
    events: EventBus,
    concurrency: Arc<ConcurrencyGate>,
    controls: Mutex<HashMap<Uuid, CancellationToken>>,
    default_output: RwLock<PathBuf>,
    thumbnail_directory: PathBuf,
}

impl DownloadManager {
    pub fn new(
        storage: Arc<Storage>,
        runtime: RuntimeManager,
        events: EventBus,
        concurrency: usize,
        default_output: PathBuf,
        thumbnail_directory: PathBuf,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                storage,
                runtime,
                events,
                concurrency: Arc::new(ConcurrencyGate::new(concurrency)),
                controls: Mutex::new(HashMap::new()),
                default_output: RwLock::new(default_output),
                thumbnail_directory,
            }),
        }
    }

    async fn enqueue(&self, mut job: DownloadJob) -> Result<DownloadJob, FetchError> {
        job.transition(DownloadStatus::Queued)?;
        job.error_code = None;
        job.error_message = None;
        self.inner
            .storage
            .update_job(&job)
            .await
            .map_err(storage_error)?;
        self.spawn(job.id).await;
        Ok(job)
    }

    async fn spawn(&self, id: Uuid) {
        let token = CancellationToken::new();
        self.inner.controls.lock().await.insert(id, token.clone());
        let inner = self.inner.clone();
        tokio::spawn(async move {
            if let Err(failure) = run_job(inner.clone(), id, token).await {
                error!(job_id = %id, error = %failure.public_message(), "download job execution failed");
                if let Err(error) = fail_job(&inner, id, &failure).await {
                    error!(job_id = %id, %error, "could not persist job failure");
                }
            }
            inner.controls.lock().await.remove(&id);
        });
    }

    pub async fn shutdown(&self) {
        loop {
            let tokens = self
                .inner
                .controls
                .lock()
                .await
                .values()
                .cloned()
                .collect::<Vec<_>>();
            if tokens.is_empty() {
                return;
            }
            for token in tokens {
                token.cancel();
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    }
}

#[async_trait::async_trait]
impl DownloadOperations for DownloadManager {
    async fn create(&self, mut request: DownloadRequest) -> Result<DownloadJob, FetchError> {
        if request.url.trim().is_empty() {
            return Err(FetchError::InvalidRequest("URL must not be empty".into()));
        }
        if request
            .duration_seconds
            .is_some_and(|duration| !duration.is_finite() || duration < 0.0)
        {
            return Err(FetchError::InvalidRequest(
                "duration must be a non-negative number".into(),
            ));
        }
        let output_root = request
            .output_directory
            .clone()
            .unwrap_or(self.inner.default_output.read().await.clone());
        let output = match request.playlist.as_ref() {
            Some(playlist) => playlist_output_directory(&output_root, playlist)?,
            None => output_root,
        };
        validate_output_directory(&output).await?;
        request.output_directory = Some(output);
        let mut job = DownloadJob::new(request);
        job.transition(DownloadStatus::Queued)?;
        self.inner
            .storage
            .insert_job(&job)
            .await
            .map_err(storage_error)?;
        self.inner
            .events
            .publish(ApplicationEvent::DownloadCreated(job.clone()));
        self.spawn(job.id).await;
        Ok(job)
    }

    async fn list(&self) -> Result<Vec<DownloadJob>, FetchError> {
        self.inner.storage.list_jobs().await.map_err(storage_error)
    }

    async fn get(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
        self.inner
            .storage
            .get_job(id)
            .await
            .map_err(storage_error)?
            .ok_or(FetchError::NotFound)
    }

    async fn stop(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
        let token = self.inner.controls.lock().await.get(&id).cloned().ok_or(
            FetchError::InvalidTransition {
                from: self.get(id).await?.status,
                to: DownloadStatus::Stopped,
            },
        )?;
        token.cancel();
        // The process owner persists STOPPED after the child has exited.
        self.get(id).await
    }

    async fn resume(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
        let job = self.get(id).await?;
        if job.status != DownloadStatus::Stopped {
            return Err(FetchError::InvalidTransition {
                from: job.status,
                to: DownloadStatus::Queued,
            });
        }
        self.enqueue(job).await
    }

    async fn retry(&self, id: Uuid) -> Result<DownloadJob, FetchError> {
        let job = self.get(id).await?;
        if job.status != DownloadStatus::Failed {
            return Err(FetchError::InvalidTransition {
                from: job.status,
                to: DownloadStatus::Queued,
            });
        }
        self.enqueue(job).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), FetchError> {
        if self.inner.controls.lock().await.contains_key(&id) {
            return Err(FetchError::InvalidTransition {
                from: self.get(id).await?.status,
                to: DownloadStatus::Stopped,
            });
        }
        remove_cached_thumbnail(&self.inner, id).await;
        if self
            .inner
            .storage
            .delete_job(id)
            .await
            .map_err(storage_error)?
        {
            Ok(())
        } else {
            Err(FetchError::NotFound)
        }
    }

    async fn update_defaults(
        &self,
        download_directory: PathBuf,
        concurrent_downloads: u8,
    ) -> Result<(), FetchError> {
        validate_output_directory(&download_directory).await?;
        *self.inner.default_output.write().await = download_directory;
        self.inner
            .concurrency
            .set_limit(concurrent_downloads as usize);
        Ok(())
    }
}

struct ConcurrencyGate {
    limit: AtomicUsize,
    active: AtomicUsize,
    changed: Notify,
}

impl ConcurrencyGate {
    fn new(limit: usize) -> Self {
        Self {
            limit: AtomicUsize::new(limit.clamp(1, 16)),
            active: AtomicUsize::new(0),
            changed: Notify::new(),
        }
    }

    fn set_limit(&self, limit: usize) {
        self.limit.store(limit.clamp(1, 16), Ordering::Release);
        self.changed.notify_waiters();
    }

    async fn acquire(self: Arc<Self>) -> ConcurrencyPermit {
        loop {
            let notified = self.changed.notified();
            let active = self.active.load(Ordering::Acquire);
            let limit = self.limit.load(Ordering::Acquire);
            if active < limit
                && self
                    .active
                    .compare_exchange(active, active + 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                return ConcurrencyPermit { gate: self.clone() };
            }
            notified.await;
        }
    }
}

struct ConcurrencyPermit {
    gate: Arc<ConcurrencyGate>,
}

impl Drop for ConcurrencyPermit {
    fn drop(&mut self) {
        self.gate.active.fetch_sub(1, Ordering::AcqRel);
        self.gate.changed.notify_waiters();
    }
}

enum ProcessOutcome {
    Exited(ExitStatus),
    Cancelled,
}

async fn run_job(
    inner: Arc<Inner>,
    id: Uuid,
    cancellation: CancellationToken,
) -> Result<(), FetchError> {
    let permit = tokio::select! {
        permit = inner.concurrency.clone().acquire() => permit,
        _ = cancellation.cancelled() => {
            stop_job(&inner, id).await?;
            return Ok(());
        }
    };
    let mut job = inner
        .storage
        .get_job(id)
        .await
        .map_err(storage_error)?
        .ok_or(FetchError::NotFound)?;
    job.transition(DownloadStatus::Downloading)?;
    inner
        .storage
        .update_job(&job)
        .await
        .map_err(storage_error)?;
    inner
        .events
        .publish(ApplicationEvent::DownloadProgress(job.clone()));

    let ytdlp_path = inner.runtime.ytdlp_path().await?;
    let mut adapter = YtDlp::new(ytdlp_path);
    if let Some(directory) = inner.runtime.ffmpeg_directory().await {
        adapter = adapter.with_ffmpeg_directory(directory);
    }
    tokio::fs::create_dir_all(&inner.thumbnail_directory)
        .await
        .map_err(|error| {
            FetchError::Internal(format!("could not create thumbnail cache: {error}"))
        })?;
    let artifacts = DownloadArtifacts {
        thumbnail_directory: inner.thumbnail_directory.clone(),
        thumbnail_stem: job.id.to_string(),
    };
    let mut child = adapter
        .download_command_with_artifacts(&job.request, Some(&artifacts))?
        .spawn()
        .map_err(|error| FetchError::ProcessFailed {
            summary: "could not start yt-dlp".into(),
            details: error.to_string(),
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| FetchError::Internal("yt-dlp stdout was not captured".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| FetchError::Internal("yt-dlp stderr was not captured".into()))?;
    let (sender, mut receiver) = mpsc::channel::<(bool, String)>(128);
    spawn_line_reader(stdout, false, sender.clone());
    spawn_line_reader(stderr, true, sender);
    let mut completed_path = None;
    let mut diagnostics = String::new();

    let outcome = {
        let wait = child.wait();
        tokio::pin!(wait);
        loop {
            tokio::select! {
                _ = cancellation.cancelled() => break ProcessOutcome::Cancelled,
                result = &mut wait => break ProcessOutcome::Exited(result.map_err(|error| FetchError::ProcessFailed { summary: "could not wait for yt-dlp".into(), details: error.to_string() })?),
                line = receiver.recv() => if let Some((is_stderr, line)) = line {
                    handle_line(&inner, &mut job, &mut completed_path, &mut diagnostics, is_stderr, line).await?;
                },
            }
        }
    };

    if matches!(outcome, ProcessOutcome::Cancelled) {
        #[cfg(unix)]
        if let Some(process_id) = child.id() {
            // The adapter starts each job in a dedicated process group. A negative
            // PID targets that group so yt-dlp post-processors are not orphaned.
            // SAFETY: kill is called with a valid process-group identifier and a
            // constant signal; no Rust memory is read or written by this call.
            unsafe {
                libc::kill(-(process_id as i32), libc::SIGTERM);
            }
        }
        #[cfg(windows)]
        if let Some(process_id) = child.id() {
            // /T terminates yt-dlp and its FFmpeg descendants. Arguments are
            // passed structurally and never through a command shell.
            let process_id = process_id.to_string();
            let _ = tokio::process::Command::new("taskkill")
                .args(["/PID", process_id.as_str(), "/T", "/F"])
                .status()
                .await;
        }
        if let Err(error) = child.start_kill()
            && !matches!(child.try_wait(), Ok(Some(_)))
        {
            return Err(FetchError::ProcessFailed {
                summary: "could not stop yt-dlp".into(),
                details: error.to_string(),
            });
        }
        let _ = child.wait().await;
        drop(permit);
        stop_job(&inner, id).await?;
        return Ok(());
    }
    while let Ok((is_stderr, line)) = receiver.try_recv() {
        handle_line(
            &inner,
            &mut job,
            &mut completed_path,
            &mut diagnostics,
            is_stderr,
            line,
        )
        .await?;
    }
    drop(permit);
    let ProcessOutcome::Exited(status) = outcome else {
        unreachable!()
    };
    if !status.success() {
        return Err(map_process_error(&diagnostics));
    }
    let output_path = completed_path.ok_or_else(|| FetchError::ProcessFailed {
        summary: "yt-dlp exited successfully but did not report an output file".into(),
        details: diagnostics.clone(),
    })?;
    let mut file = validate_completed_path(&job, output_path).await?;
    file.thumbnail_path = cached_thumbnail_path(&inner.thumbnail_directory, job.id).await;
    file.thumbnail_available = file.thumbnail_path.is_some();
    inner
        .storage
        .insert_completed_file(&file)
        .await
        .map_err(storage_error)?;
    job.transition(DownloadStatus::Completed)?;
    job.progress.progress_percent = Some(100.0);
    inner
        .storage
        .update_job(&job)
        .await
        .map_err(storage_error)?;
    inner
        .events
        .publish(ApplicationEvent::DownloadCompleted(job.clone()));
    inner
        .events
        .publish(ApplicationEvent::CompletedFileCreated(file.clone()));
    info!(job_id = %id, file_id = %file.id, path = %file.path.display(), "download completed");
    Ok(())
}

fn spawn_line_reader(
    stream: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    stderr: bool,
    sender: mpsc::Sender<(bool, String)>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stream).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if sender.send((stderr, line)).await.is_err() {
                break;
            }
        }
    });
}

async fn handle_line(
    inner: &Inner,
    job: &mut DownloadJob,
    completed_path: &mut Option<PathBuf>,
    diagnostics: &mut String,
    is_stderr: bool,
    line: String,
) -> Result<(), FetchError> {
    match parse_process_line(&line) {
        ProcessLine::Progress(progress) => {
            job.progress = progress;
            job.updated_at = Utc::now();
            inner.storage.update_job(job).await.map_err(storage_error)?;
            inner
                .events
                .publish(ApplicationEvent::DownloadProgress(job.clone()));
        }
        ProcessLine::Postprocessing if job.status == DownloadStatus::Downloading => {
            job.transition(DownloadStatus::Postprocessing)?;
            inner.storage.update_job(job).await.map_err(storage_error)?;
            inner
                .events
                .publish(ApplicationEvent::DownloadPostprocessing(job.clone()));
        }
        ProcessLine::CompletedFile(path) => *completed_path = Some(path),
        ProcessLine::Diagnostic(line) => {
            if is_stderr {
                if diagnostics.len() < 65_536 {
                    diagnostics.push_str(&line);
                    diagnostics.push('\n');
                }
                warn!(target: "ytdlp", job_id = %job.id, "{line}");
            } else {
                info!(target: "ytdlp", job_id = %job.id, "{line}");
            }
            inner
                .storage
                .append_log(
                    if is_stderr { "warn" } else { "info" },
                    "yt-dlp",
                    &line,
                    Some(&job.id.to_string()),
                )
                .await
                .map_err(storage_error)?;
        }
        ProcessLine::Postprocessing => {}
    }
    Ok(())
}

async fn stop_job(inner: &Inner, id: Uuid) -> Result<(), FetchError> {
    let mut job = inner
        .storage
        .get_job(id)
        .await
        .map_err(storage_error)?
        .ok_or(FetchError::NotFound)?;
    if job.status != DownloadStatus::Stopped {
        job.transition(DownloadStatus::Stopped)?;
        inner
            .storage
            .update_job(&job)
            .await
            .map_err(storage_error)?;
        inner.events.publish(ApplicationEvent::DownloadStopped(job));
    }
    remove_cached_thumbnail(inner, id).await;
    Ok(())
}

async fn fail_job(inner: &Inner, id: Uuid, failure: &FetchError) -> Result<(), FetchError> {
    let mut job = inner
        .storage
        .get_job(id)
        .await
        .map_err(storage_error)?
        .ok_or(FetchError::NotFound)?;
    if matches!(
        job.status,
        DownloadStatus::Completed | DownloadStatus::Stopped | DownloadStatus::Failed
    ) {
        return Ok(());
    }
    job.transition(DownloadStatus::Failed)?;
    job.error_code = serde_json::to_value(failure.code())
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned));
    job.error_message = Some(failure.public_message());
    inner
        .storage
        .append_log(
            "error",
            "download",
            &failure.public_message(),
            failure.diagnostic_details(),
        )
        .await
        .map_err(storage_error)?;
    inner
        .storage
        .update_job(&job)
        .await
        .map_err(storage_error)?;
    inner.events.publish(ApplicationEvent::DownloadFailed(job));
    remove_cached_thumbnail(inner, id).await;
    Ok(())
}

async fn remove_cached_thumbnail(inner: &Inner, id: Uuid) {
    let path = inner.thumbnail_directory.join(format!("{id}.jpg"));
    if let Err(error) = tokio::fs::remove_file(&path).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        warn!(job_id = %id, path = %path.display(), %error, "could not remove cached thumbnail");
    }
}

async fn validate_output_directory(path: &std::path::Path) -> Result<(), FetchError> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|error| FetchError::OutputDirectoryUnavailable(error.to_string()))?;
    if !tokio::fs::metadata(path)
        .await
        .map_err(|error| FetchError::OutputDirectoryUnavailable(error.to_string()))?
        .is_dir()
    {
        return Err(FetchError::OutputDirectoryUnavailable(format!(
            "{} is not a directory",
            path.display()
        )));
    }
    let probe = path.join(format!(".fetch-write-test-{}", Uuid::new_v4()));
    tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
        .await
        .map_err(|error| FetchError::OutputDirectoryUnavailable(error.to_string()))?;
    tokio::fs::remove_file(&probe)
        .await
        .map_err(|error| FetchError::OutputDirectoryUnavailable(error.to_string()))?;
    Ok(())
}

fn playlist_output_directory(
    root: &std::path::Path,
    playlist: &fetch_core::PlaylistContext,
) -> Result<PathBuf, FetchError> {
    if playlist.title.trim().is_empty() || playlist.id.trim().is_empty() || playlist.index == 0 {
        return Err(FetchError::InvalidRequest(
            "playlist title, ID, and one-based item index are required".into(),
        ));
    }
    let title = sanitize_path_component(&playlist.title, "Playlist", 100);
    let id = sanitize_path_component(&playlist.id, "playlist", 48);
    Ok(root.join("Playlists").join(format!("{title} [{id}]")))
}

fn sanitize_path_component(value: &str, fallback: &str, max_chars: usize) -> String {
    let mut result = String::new();
    let mut previous_space = false;
    for character in value.trim().chars() {
        let invalid = character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            );
        let character = if invalid { '_' } else { character };
        if character.is_whitespace() {
            if !previous_space {
                result.push(' ');
            }
            previous_space = true;
        } else {
            result.push(character);
            previous_space = false;
        }
        if result.chars().count() >= max_chars {
            break;
        }
    }
    let mut result = result.trim_matches([' ', '.']).to_owned();
    if result.is_empty() || matches!(result.as_str(), "." | "..") {
        result = fallback.to_owned();
    }
    let upper = result.to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.as_bytes()[3].is_ascii_digit()
            && upper.as_bytes()[3] != b'0');
    if reserved {
        result.insert(0, '_');
    }
    result
}

async fn cached_thumbnail_path(directory: &std::path::Path, job_id: Uuid) -> Option<PathBuf> {
    let root = tokio::fs::canonicalize(directory).await.ok()?;
    let candidate = tokio::fs::canonicalize(directory.join(format!("{job_id}.jpg")))
        .await
        .ok()?;
    let metadata = tokio::fs::metadata(&candidate).await.ok()?;
    (metadata.is_file() && candidate.starts_with(root)).then_some(candidate)
}

async fn validate_completed_path(
    job: &DownloadJob,
    path: PathBuf,
) -> Result<CompletedFile, FetchError> {
    let canonical = tokio::fs::canonicalize(&path)
        .await
        .map_err(|_| FetchError::FileNotFound)?;
    let output = tokio::fs::canonicalize(
        job.request
            .output_directory
            .as_ref()
            .ok_or_else(|| FetchError::Internal("job has no output directory".into()))?,
    )
    .await
    .map_err(|error| FetchError::OutputDirectoryUnavailable(error.to_string()))?;
    if !canonical.starts_with(&output) {
        return Err(FetchError::ProcessFailed {
            summary: "yt-dlp reported a file outside the configured output directory".into(),
            details: canonical.display().to_string(),
        });
    }
    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|_| FetchError::FileNotFound)?;
    if !metadata.is_file() {
        return Err(FetchError::FileNotFound);
    }
    let filename = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(FetchError::FileNotFound)?
        .to_owned();
    let mime = mime_guess::from_path(&canonical)
        .first_or_octet_stream()
        .to_string();
    let browser_playable = matches!(
        mime.as_str(),
        "video/mp4" | "video/webm" | "audio/mpeg" | "audio/ogg" | "audio/wav" | "audio/mp4"
    );
    Ok(CompletedFile {
        id: Uuid::new_v4(),
        job_id: job.id,
        playlist: job.request.playlist.clone(),
        filename,
        path: canonical,
        thumbnail_path: None,
        thumbnail_available: false,
        size_bytes: metadata.len(),
        mime_type: mime,
        title: job.request.title.clone(),
        browser_playable,
        playback: None,
        created_at: Utc::now(),
    })
}

fn storage_error(error: fetch_storage::StorageError) -> FetchError {
    FetchError::Internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn concurrency_gate_applies_limit_changes_without_restart() {
        let gate = Arc::new(ConcurrencyGate::new(1));
        let first = gate.clone().acquire().await;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), gate.clone().acquire())
                .await
                .is_err()
        );

        gate.set_limit(2);
        let second = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            gate.clone().acquire(),
        )
        .await
        .expect("increased limit should release a queued job");
        gate.set_limit(1);
        drop(first);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), gate.clone().acquire())
                .await
                .is_err()
        );
        drop(second);
        tokio::time::timeout(std::time::Duration::from_millis(100), gate.acquire())
            .await
            .expect("reduced limit should apply after active jobs finish");
    }
    use fetch_core::DownloadMode;

    #[cfg(unix)]
    #[tokio::test]
    async fn fixture_download_runs_through_queue_and_persists_file() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let runtime_paths = fetch_runtime::RuntimePaths::new(temp.path().join("runtime"));
        std::fs::create_dir_all(runtime_paths.ytdlp_directory()).unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/fake-ytdlp.sh"),
            runtime_paths.ytdlp_executable(),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(runtime_paths.ytdlp_executable())
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(runtime_paths.ytdlp_executable(), permissions).unwrap();

        let storage = Arc::new(
            Storage::open(std::path::Path::new(":memory:"))
                .await
                .unwrap(),
        );
        let events = EventBus::default();
        let mut event_receiver = events.subscribe();
        let manager = DownloadManager::new(
            storage.clone(),
            RuntimeManager::new(runtime_paths).unwrap(),
            events,
            1,
            temp.path().join("downloads"),
            temp.path().join("thumbnails"),
        );
        let job = manager
            .create(DownloadRequest {
                url: "https://example.test/media".into(),
                title: Some("Fixture".into()),
                duration_seconds: Some(42.0),
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
        for _ in 0..100 {
            let current = manager.get(job.id).await.unwrap();
            if matches!(
                current.status,
                DownloadStatus::Completed | DownloadStatus::Failed
            ) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let completed = manager.get(job.id).await.unwrap();
        assert_eq!(
            completed.status,
            DownloadStatus::Completed,
            "{:?}",
            completed.error_message
        );
        let files = storage.list_completed_files().await.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].path.is_file());
        assert!(files[0].thumbnail_available);
        assert!(files[0].thumbnail_path.as_ref().unwrap().is_file());
        let mut saw_postprocessing = false;
        let mut saw_completed_file = false;
        while let Ok(event) = event_receiver.try_recv() {
            saw_postprocessing |= matches!(event, ApplicationEvent::DownloadPostprocessing(_));
            saw_completed_file |= matches!(event, ApplicationEvent::CompletedFileCreated(_));
        }
        assert!(
            saw_postprocessing,
            "post-processing was not emitted over the event bus"
        );
        assert!(
            saw_completed_file,
            "completed file was not emitted over the event bus"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn shutdown_terminates_owned_fixture_process_and_persists_state() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let runtime_paths = fetch_runtime::RuntimePaths::new(temp.path().join("runtime"));
        std::fs::create_dir_all(runtime_paths.ytdlp_directory()).unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/fake-ytdlp.sh"),
            runtime_paths.ytdlp_executable(),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(runtime_paths.ytdlp_executable())
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(runtime_paths.ytdlp_executable(), permissions).unwrap();
        let storage = Arc::new(
            Storage::open(std::path::Path::new(":memory:"))
                .await
                .unwrap(),
        );
        let manager = DownloadManager::new(
            storage,
            RuntimeManager::new(runtime_paths).unwrap(),
            EventBus::default(),
            1,
            temp.path().join("downloads"),
            temp.path().join("thumbnails"),
        );
        let job = manager
            .create(DownloadRequest {
                url: "https://example.test/slow".into(),
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
            })
            .await
            .unwrap();
        for _ in 0..50 {
            if manager.get(job.id).await.unwrap().status == DownloadStatus::Downloading {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        tokio::time::timeout(std::time::Duration::from_secs(2), manager.shutdown())
            .await
            .expect("download manager did not stop during graceful shutdown");
        assert_eq!(
            manager.get(job.id).await.unwrap().status,
            DownloadStatus::Stopped
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn queue_enforces_the_configured_concurrency_limit() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let runtime_paths = fetch_runtime::RuntimePaths::new(temp.path().join("runtime"));
        std::fs::create_dir_all(runtime_paths.ytdlp_directory()).unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/fake-ytdlp.sh"),
            runtime_paths.ytdlp_executable(),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(runtime_paths.ytdlp_executable())
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(runtime_paths.ytdlp_executable(), permissions).unwrap();
        let storage = Arc::new(
            Storage::open(std::path::Path::new(":memory:"))
                .await
                .unwrap(),
        );
        let manager = DownloadManager::new(
            storage,
            RuntimeManager::new(runtime_paths).unwrap(),
            EventBus::default(),
            1,
            temp.path().join("downloads"),
            temp.path().join("thumbnails"),
        );
        let request = |suffix: &str| DownloadRequest {
            url: format!("https://example.test/slow-{suffix}"),
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
        };
        let first = manager.create(request("one")).await.unwrap();
        let second = manager.create(request("two")).await.unwrap();
        for _ in 0..100 {
            if manager.get(first.id).await.unwrap().status == DownloadStatus::Downloading {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            manager.get(first.id).await.unwrap().status,
            DownloadStatus::Downloading
        );
        assert_eq!(
            manager.get(second.id).await.unwrap().status,
            DownloadStatus::Queued
        );
        manager.stop(second.id).await.unwrap();
        manager.stop(first.id).await.unwrap();
        for _ in 0..100 {
            let states = [
                manager.get(first.id).await.unwrap().status,
                manager.get(second.id).await.unwrap().status,
            ];
            if states
                .iter()
                .all(|status| *status == DownloadStatus::Stopped)
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("queued jobs did not stop cleanly");
    }

    #[test]
    fn playlist_paths_are_ordered_and_cross_platform_safe() {
        let context = fetch_core::PlaylistContext {
            id: "../list:42".into(),
            title: "CON/video? collection.".into(),
            index: 7,
        };
        let path = playlist_output_directory(std::path::Path::new("downloads"), &context)
            .expect("playlist path should be valid");
        assert_eq!(
            path.parent().unwrap(),
            std::path::Path::new("downloads/Playlists")
        );
        let folder = path.file_name().unwrap().to_string_lossy();
        assert!(!folder.contains('/'));
        assert!(!folder.contains(':'));
        assert!(!folder.contains('?'));
        assert_eq!(sanitize_path_component("CON", "Playlist", 100), "_CON");
        assert!(
            playlist_output_directory(
                std::path::Path::new("downloads"),
                &fetch_core::PlaylistContext {
                    index: 0,
                    ..context
                }
            )
            .is_err()
        );
    }
}
