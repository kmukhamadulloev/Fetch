//! Persistent single-worker queue for metadata and explicit local exports.
use crate::{
    metadata::{MetadataService, remove, sync_file, sync_parent},
    metadata_adapter::{io_error, revision},
    processing_adapter::ProcessingAdapter,
};
use chrono::Utc;
use fetch_core::{
    ApplicationEvent, CompletedFile, EventBus, ExportRequest, FetchError, MetadataSaveState,
    MetadataSaveStatus, MetadataUpdate, ProcessingCapabilities, ProcessingJob, ProcessingKind,
    ProcessingState,
};
use fetch_storage::Storage;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, Notify, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub struct ProcessingManager {
    inner: Arc<Inner>,
}
struct Inner {
    storage: Arc<Storage>,
    metadata: MetadataService,
    adapter: ProcessingAdapter,
    events: EventBus,
    control: Mutex<Control>,
    notify: Notify,
    shutdown: CancellationToken,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    failed: std::sync::Mutex<bool>,
}
#[derive(Default)]
struct Control {
    active: Option<(Uuid, CancellationToken)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Work {
    Metadata {
        update: MetadataUpdate,
    },
    Export {
        request: ExportRequest,
        root: PathBuf,
        temporary: Option<PathBuf>,
        output: Option<PathBuf>,
        #[serde(default)]
        cancel_requested: bool,
    },
}
impl Work {
    fn json(&self) -> Result<String, FetchError> {
        serde_json::to_string(self).map_err(io_error)
    }
}
fn metadata_status(job: &ProcessingJob) -> MetadataSaveStatus {
    MetadataSaveStatus {
        file_id: job.source_file_id,
        operation_id: Some(job.id),
        state: match job.state {
            ProcessingState::Queued | ProcessingState::Running => MetadataSaveState::Saving,
            ProcessingState::Completed => MetadataSaveState::Completed,
            _ => MetadataSaveState::Failed,
        },
        error: job.error.clone(),
    }
}

impl ProcessingManager {
    pub async fn new(
        storage: Arc<Storage>,
        metadata: MetadataService,
        adapter: ProcessingAdapter,
        events: EventBus,
        shutdown: CancellationToken,
    ) -> Result<Self, FetchError> {
        let service = Self {
            inner: Arc::new(Inner {
                storage,
                metadata,
                adapter,
                events,
                control: Mutex::default(),
                notify: Notify::new(),
                shutdown,
                task: Mutex::new(None),
                failed: std::sync::Mutex::new(false),
            }),
        };
        service.recover().await?;
        let worker = service.clone();
        *service.inner.task.lock().await = Some(tokio::spawn(async move {
            worker.run().await;
        }));
        Ok(service)
    }
    pub async fn shutdown(&self) {
        self.inner.shutdown.cancel();
        self.inner.notify.notify_one();
        if let Some(task) = self.inner.task.lock().await.take()
            && let Err(error) = task.await
        {
            tracing::error!(%error,"processing worker stopped unexpectedly");
        }
    }
    async fn file(&self, id: Uuid) -> Result<CompletedFile, FetchError> {
        self.inner
            .storage
            .get_completed_file(id)
            .await
            .map_err(io_error)?
            .ok_or(FetchError::FileNotFound)
    }
    async fn records(&self) -> Result<Vec<(ProcessingJob, Work)>, FetchError> {
        self.inner
            .storage
            .processing_jobs()
            .await
            .map_err(io_error)?
            .into_iter()
            .map(|(j, s)| Ok((j, serde_json::from_str(&s).map_err(io_error)?)))
            .collect()
    }
    pub async fn list(&self) -> Result<Vec<ProcessingJob>, FetchError> {
        let mut jobs = self
            .records()
            .await?
            .into_iter()
            .map(|(j, _)| j)
            .collect::<Vec<_>>();
        jobs.reverse();
        Ok(jobs)
    }
    pub async fn capabilities(&self, id: Uuid) -> Result<ProcessingCapabilities, FetchError> {
        // Read-only probing may overlap an export. A replacement race is detected
        // by the revision check at admission and again by the worker.
        self.inner
            .adapter
            .capabilities(&self.file(id).await?.path)
            .await
    }
    async fn persist(&self, job: &ProcessingJob, work: &Work) -> Result<(), FetchError> {
        self.inner
            .storage
            .update_processing_job(job, &work.json()?)
            .await
            .map_err(io_error)?;
        self.publish(job).await;
        Ok(())
    }
    async fn publish(&self, job: &ProcessingJob) {
        self.inner
            .events
            .publish(ApplicationEvent::ProcessingUpdated(job.clone()));
        if job.kind == ProcessingKind::Metadata {
            self.inner.metadata.publish(metadata_status(job)).await;
        }
    }
    async fn enqueue(&self, job: ProcessingJob, work: Work) -> Result<ProcessingJob, FetchError> {
        if *self.inner.failed.lock().expect("worker health lock") {
            return Err(FetchError::Conflict(
                "Processing stopped after a storage error. Restart Fetch to recover queued work."
                    .into(),
            ));
        }
        if self.inner.shutdown.is_cancelled() {
            return Err(FetchError::Conflict("Fetch is shutting down".into()));
        }
        self.inner
            .storage
            .insert_processing_job(&job, &work.json()?)
            .await
            .map_err(io_error)?;
        self.publish(&job).await;
        self.inner.notify.notify_one();
        Ok(job)
    }
    pub async fn start_export(
        &self,
        id: Uuid,
        request: ExportRequest,
    ) -> Result<ProcessingJob, FetchError> {
        request.validate()?;
        // Admission and dequeue are serialized; source revision is rechecked by the worker.
        let _control = self.inner.control.lock().await;
        let file = self.file(id).await?;
        if revision(&file.path).await? != request.revision {
            return Err(crate::metadata::stale());
        }
        if !request.acknowledge_omissions {
            return Err(FetchError::InvalidRequest(
                "Review and acknowledge export limitations".into(),
            ));
        }
        let settings = self
            .inner
            .storage
            .load_settings()
            .await
            .map_err(io_error)?
            .ok_or(FetchError::NotFound)?;
        let root = tokio::fs::canonicalize(settings.download_directory)
            .await
            .map_err(io_error)?;
        if !root.is_dir() {
            return Err(FetchError::InvalidRequest(
                "The download directory is unavailable".into(),
            ));
        }
        self.enqueue(
            ProcessingJob::new(request.kind(), id, file.title.unwrap_or(file.filename)),
            Work::Export {
                request,
                root,
                temporary: None,
                output: None,
                cancel_requested: false,
            },
        )
        .await
    }
    pub async fn start_metadata(
        &self,
        id: Uuid,
        update: MetadataUpdate,
    ) -> Result<MetadataSaveStatus, FetchError> {
        update.validate()?;
        let _control = self.inner.control.lock().await;
        if self.records().await?.iter().any(|(j, _)| {
            j.source_file_id == id && j.kind == ProcessingKind::Metadata && !j.state.terminal()
        }) {
            return Err(crate::metadata::busy());
        }
        let file = self.file(id).await?;
        if revision(&file.path).await? != update.revision {
            return Err(crate::metadata::stale());
        }
        if tokio::fs::metadata(&file.path)
            .await
            .map_err(io_error)?
            .permissions()
            .readonly()
        {
            return Err(FetchError::InvalidRequest(
                "This media file is read-only".into(),
            ));
        }
        let job = self
            .enqueue(
                ProcessingJob::new(
                    ProcessingKind::Metadata,
                    id,
                    file.title.unwrap_or(file.filename),
                ),
                Work::Metadata { update },
            )
            .await?;
        Ok(metadata_status(&job))
    }
    pub async fn metadata_status(&self, id: Uuid) -> Result<MetadataSaveStatus, FetchError> {
        self.file(id).await?;
        Ok(self
            .records()
            .await?
            .iter()
            .rev()
            .find(|(j, _)| j.source_file_id == id && j.kind == ProcessingKind::Metadata)
            .map(|(j, _)| metadata_status(j))
            .unwrap_or(MetadataSaveStatus {
                file_id: id,
                operation_id: None,
                state: MetadataSaveState::Idle,
                error: None,
            }))
    }
    pub async fn cancel(&self, id: Uuid) -> Result<ProcessingJob, FetchError> {
        let control = self.inner.control.lock().await;
        let (mut job, mut work) = self
            .records()
            .await?
            .into_iter()
            .find(|(j, _)| j.id == id)
            .ok_or(FetchError::NotFound)?;
        if !job.can_cancel() {
            return Err(FetchError::Conflict(
                "This process can no longer be cancelled".into(),
            ));
        }
        if job.state == ProcessingState::Queued {
            job.transition(ProcessingState::Cancelled)?;
            job.stage = "cancelled".into();
        } else if let Work::Export {
            cancel_requested, ..
        } = &mut work
        {
            *cancel_requested = true;
        }
        self.persist(&job, &work).await?;
        if let Some((active, token)) = &control.active
            && *active == id
        {
            token.cancel();
        }
        Ok(job)
    }
    pub async fn retry(&self, id: Uuid) -> Result<ProcessingJob, FetchError> {
        let (job, work) = self
            .records()
            .await?
            .into_iter()
            .find(|(j, _)| j.id == id)
            .ok_or(FetchError::NotFound)?;
        if !job.can_retry() {
            return Err(FetchError::Conflict(
                "This process cannot be retried".into(),
            ));
        }
        let Work::Export { mut request, .. } = work else {
            return Err(FetchError::Conflict(
                "Metadata saves must be reopened in the editor".into(),
            ));
        };
        request.revision = revision(&self.file(job.source_file_id).await?.path).await?;
        self.start_export(job.source_file_id, request).await
    }
    async fn recover(&self) -> Result<(), FetchError> {
        for (mut job, work) in self.records().await? {
            self.cleanup(&job, &work).await?;
            if job.state == ProcessingState::Running {
                let cancelled = matches!(
                    &work,
                    Work::Export {
                        cancel_requested: true,
                        ..
                    }
                );
                job.transition(if cancelled {
                    ProcessingState::Cancelled
                } else {
                    ProcessingState::Interrupted
                })?;
                job.stage = if cancelled {
                    "cancelled"
                } else {
                    "interrupted"
                }
                .into();
                job.error=Some("Processing was interrupted. Retry an export from the beginning; reopen metadata to check saved fields.".into());
                self.persist(&job, &work).await?;
            } else {
                self.publish(&job).await;
            }
        }
        Ok(())
    }
    async fn cleanup(&self, job: &ProcessingJob, work: &Work) -> Result<(), FetchError> {
        if let Work::Export {
            temporary: Some(temp),
            output,
            ..
        } = work
        {
            if !tokio::fs::try_exists(temp).await.map_err(io_error)? {
                return Ok(());
            }
            if job.state != ProcessingState::Completed
                && let Some(output) = output
                && tokio::fs::try_exists(temp).await.map_err(io_error)?
                && tokio::fs::try_exists(output).await.map_err(io_error)?
            {
                // Hard-link identity proves this destination belongs to the journal.
                if same_file::is_same_file(temp, output).map_err(io_error)? {
                    remove(output).await?;
                    sync_parent(output).await?;
                }
            }
            remove(temp).await?;
            sync_parent(temp).await?;
        }
        Ok(())
    }
    async fn run(&self) {
        loop {
            if self.inner.shutdown.is_cancelled() {
                break;
            }
            let notified = self.inner.notify.notified();
            match self.next().await {
                Ok(Some((job, work, token, guard))) => {
                    let _guard = guard;
                    if let Err(error) = self.execute(job, work, token).await {
                        tracing::error!(reason=%error.public_message(),"processing persistence/recovery failed; worker stopped");
                        *self.inner.failed.lock().expect("worker health lock") = true;
                        break;
                    }
                }
                Ok(None) => {
                    tokio::select! {_=notified=>{},_=self.inner.shutdown.cancelled()=>break}
                }
                Err(error) => {
                    tracing::error!(reason=%error.public_message(),"could not read processing queue; worker stopped");
                    *self.inner.failed.lock().expect("worker health lock") = true;
                    break;
                }
            }
        }
    }
    async fn next(
        &self,
    ) -> Result<
        Option<(
            ProcessingJob,
            Work,
            CancellationToken,
            tokio::sync::OwnedMutexGuard<()>,
        )>,
        FetchError,
    > {
        // Do not hold control while waiting for media inspection to finish.
        let guard = tokio::select! {g=self.inner.metadata.lock.clone().lock_owned()=>g,_=self.inner.shutdown.cancelled()=>return Ok(None)};
        let mut control = self.inner.control.lock().await;
        let Some((mut job, work)) = self
            .records()
            .await?
            .into_iter()
            .find(|(j, _)| j.state == ProcessingState::Queued)
        else {
            return Ok(None);
        };
        if self.inner.shutdown.is_cancelled() {
            return Ok(None);
        }
        job.transition(ProcessingState::Running)?;
        job.stage = "processing".into();
        self.persist(&job, &work).await?;
        let token = CancellationToken::new();
        control.active = Some((job.id, token.clone()));
        Ok(Some((job, work, token, guard)))
    }
    async fn execute(
        &self,
        mut job: ProcessingJob,
        mut work: Work,
        token: CancellationToken,
    ) -> Result<(), FetchError> {
        tracing::info!(process_id=%job.id,kind=?job.kind,"processing started");
        let result = match work.clone() {
            Work::Metadata { update } => match self.file(job.source_file_id).await {
                Ok(file) => self.inner.metadata.save(file, update).await.map(Some),
                Err(e) => Err(e),
            },
            Work::Export { .. } => self.export(&mut job, &mut work, &token).await,
        };
        let mut control = self.inner.control.lock().await;
        // Cancellation and publication race through this lock. A completed publication wins.
        let stored = self
            .records()
            .await?
            .into_iter()
            .find(|(j, _)| j.id == job.id)
            .ok_or(FetchError::NotFound)?;
        if stored.0.state == ProcessingState::Completed {
            self.cleanup(&stored.0, &stored.1).await?;
            control.active = None;
            return Ok(());
        }
        if let Work::Export {
            cancel_requested, ..
        } = &mut work
        {
            *cancel_requested = matches!(
                &stored.1,
                Work::Export {
                    cancel_requested: true,
                    ..
                }
            );
        }
        match result {
            Ok(Some(file)) => {
                job.transition(ProcessingState::Completed)?;
                job.stage = "completed".into();
                job.output_file_id = Some(file.id);
                self.inner
                    .events
                    .publish(ApplicationEvent::CompletedFileCreated(file));
            }
            Ok(None) => return Err(io_error("Export completed without publication")),
            Err(error) => {
                let state = if token.is_cancelled() {
                    ProcessingState::Cancelled
                } else if self.inner.shutdown.is_cancelled() {
                    ProcessingState::Interrupted
                } else {
                    ProcessingState::Failed
                };
                job.transition(state)?;
                job.stage = match state {
                    ProcessingState::Cancelled => "cancelled",
                    ProcessingState::Interrupted => "interrupted",
                    _ => "failed",
                }
                .into();
                job.error = Some(error.public_message());
                job.error_code = Some(error.code());
                tracing::warn!(process_id=%job.id,reason=%error.public_message(),"processing stopped");
            }
        }
        self.cleanup(&job, &work).await?;
        self.persist(&job, &work).await?;
        control.active = None;
        Ok(())
    }
    async fn export(
        &self,
        job: &mut ProcessingJob,
        work: &mut Work,
        token: &CancellationToken,
    ) -> Result<Option<CompletedFile>, FetchError> {
        let source = self.file(job.source_file_id).await?;
        let Work::Export { request, root, .. } = work else {
            unreachable!()
        };
        let request = request.clone();
        let directory = root.join(if job.kind == ProcessingKind::Edit {
            "Edits"
        } else {
            "Converted"
        });
        if revision(&source.path).await? != request.revision {
            return Err(crate::metadata::stale());
        }
        match tokio::fs::create_dir(&directory).await {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_error(e)),
        }
        let meta = tokio::fs::symlink_metadata(&directory)
            .await
            .map_err(io_error)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(FetchError::InvalidRequest(
                "The output folder must be a regular directory".into(),
            ));
        }
        let temporary = directory.join(format!(".fetch-{}.{}", job.id, request.format.extension()));
        let output = directory.join(format!(
            "{} [{}].{}",
            request.filename,
            job.id,
            request.format.extension()
        ));
        if let Work::Export {
            temporary: t,
            output: o,
            ..
        } = work
        {
            *t = Some(temporary.clone());
            *o = Some(output.clone());
        }
        {
            let _control = self.inner.control.lock().await;
            if token.is_cancelled() {
                return Err(FetchError::Conflict("Export cancelled".into()));
            }
            self.persist(job, work).await?;
        }
        let (sender, mut receiver) = mpsc::channel(8);
        let execution =
            self.inner
                .adapter
                .export(&source.path, &temporary, &request, token, sender);
        tokio::pin!(execution);
        let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
        loop {
            tokio::select! {
                result=&mut execution=>{result?;break;},
                Some(percent)=receiver.recv()=>{if last.elapsed()>=std::time::Duration::from_secs(1) {job.progress_percent=Some(percent);job.updated_at=Utc::now();let _control=self.inner.control.lock().await;
                    // Keep a durable cancellation request when progress races cancellation.
                    if !token.is_cancelled(){ self.persist(job,work).await?; }last=std::time::Instant::now();}}
            }
        }
        let _control = self.inner.control.lock().await;
        if token.is_cancelled() {
            return Err(FetchError::Conflict("Export cancelled".into()));
        }
        if self.inner.shutdown.is_cancelled() {
            return Err(FetchError::Conflict(
                "Export interrupted by shutdown".into(),
            ));
        }
        if revision(&source.path).await? != request.revision {
            return Err(crate::metadata::stale());
        }
        job.stage = "publishing".into();
        self.persist(job, work).await?;
        sync_file(&temporary).await?;
        tokio::fs::hard_link(&temporary, &output)
            .await
            .map_err(io_error)?; // atomic no-clobber publication
        sync_parent(&output).await?;
        let size = tokio::fs::metadata(&output).await.map_err(io_error)?.len();
        let file = CompletedFile {
            id: Uuid::new_v4(),
            job_id: None,
            origin: Some(job.kind),
            source_file_id: Some(source.id),
            playlist: None,
            filename: output.file_name().unwrap().to_string_lossy().into_owned(),
            path: output,
            thumbnail_path: None,
            thumbnail_available: false,
            size_bytes: size,
            mime_type: mime_guess::from_ext(request.format.extension())
                .first_or_octet_stream()
                .to_string(),
            title: Some(request.filename.clone()),
            browser_playable: !matches!(request.format, fetch_core::OutputFormat::Mkv),
            playback: None,
            created_at: Utc::now(),
        };
        let mut completed = job.clone();
        completed.transition(ProcessingState::Completed)?;
        completed.stage = "completed".into();
        completed.output_file_id = Some(file.id);
        self.inner
            .storage
            .finish_processing_job(&completed, &work.json()?, &file)
            .await
            .map_err(io_error)?;
        self.publish(&completed).await;
        self.inner
            .events
            .publish(ApplicationEvent::CompletedFileCreated(file));
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        library::tests::fixture,
        metadata_adapter::MetadataAdapter,
        processing_adapter::tests::{real_adapter, request, source},
    };
    use fetch_core::{ApplicationSettings, ArtworkUpdate, ExportQuality, OutputFormat};
    use fetch_runtime::RuntimePaths;
    async fn settings(storage: &Storage, root: &std::path::Path) {
        storage
            .save_settings(&ApplicationSettings {
                bind_address: "127.0.0.1".parse().unwrap(),
                port: 8080,
                allowed_networks: vec![],
                download_directory: root.into(),
                concurrent_downloads: 2,
                open_browser_on_start: false,
                start_with_system: false,
                ytdlp_auto_update: false,
                ytdlp_js_runtime: Default::default(),
            })
            .await
            .unwrap();
    }
    async fn manager(
        storage: Arc<Storage>,
        adapter: ProcessingAdapter,
    ) -> (ProcessingManager, MetadataService) {
        let events = EventBus::default();
        let metadata = MetadataService::new(storage.clone(), adapter.media.clone(), events.clone())
            .await
            .unwrap();
        let manager = ProcessingManager::new(
            storage,
            metadata.clone(),
            adapter.clone(),
            events,
            adapter.media.shutdown.clone(),
        )
        .await
        .unwrap();
        (manager, metadata)
    }
    fn missing(root: &std::path::Path) -> ProcessingAdapter {
        ProcessingAdapter {
            media: MetadataAdapter {
                paths: RuntimePaths::new(root.join("missing-runtime")),
                shutdown: CancellationToken::new(),
            },
        }
    }
    async fn wait(manager: &ProcessingManager, id: Uuid) -> ProcessingJob {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            loop {
                let job = manager
                    .list()
                    .await
                    .unwrap()
                    .into_iter()
                    .find(|j| j.id == id)
                    .unwrap();
                if job.state.terminal() {
                    return job;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
    fn export_request(revision: String) -> ExportRequest {
        ExportRequest {
            revision,
            filename: "Result".into(),
            format: OutputFormat::Mp4,
            quality: ExportQuality::Balanced,
            stream_copy: false,
            edits: None,
            acknowledge_omissions: true,
        }
    }
    #[tokio::test]
    async fn shared_queue_cancels_queued_work_and_reports_missing_runtime_and_stale_sources() {
        let (dir, storage, file) = fixture().await;
        settings(&storage, dir.path()).await;
        let (manager, metadata) = manager(storage.clone(), missing(dir.path())).await;
        let guard = metadata.lock.lock().await;
        let request = export_request(revision(&file.path).await.unwrap());
        let a = manager
            .start_export(file.id, request.clone())
            .await
            .unwrap();
        let b = manager
            .start_export(file.id, request.clone())
            .await
            .unwrap();
        let m = manager
            .start_metadata(
                file.id,
                MetadataUpdate {
                    revision: request.revision,
                    fields: Default::default(),
                    artwork: ArtworkUpdate::Keep,
                },
            )
            .await
            .unwrap();
        assert_eq!(
            manager
                .list()
                .await
                .unwrap()
                .iter()
                .filter(|j| j.state == ProcessingState::Queued)
                .count(),
            3
        );
        assert!(manager.cancel(m.operation_id.unwrap()).await.is_err());
        assert_eq!(
            manager.cancel(b.id).await.unwrap().state,
            ProcessingState::Cancelled
        );
        drop(guard);
        assert_eq!(wait(&manager, a.id).await.state, ProcessingState::Failed);
        assert_eq!(
            wait(&manager, m.operation_id.unwrap()).await.state,
            ProcessingState::Failed
        );
        assert_eq!(tokio::fs::read(&file.path).await.unwrap(), b"media");
        let guard = metadata.lock.lock().await;
        let retry = manager.retry(b.id).await.unwrap();
        assert_ne!(retry.id, b.id);
        tokio::fs::write(&file.path, b"changed source")
            .await
            .unwrap();
        drop(guard);
        let failed = wait(&manager, retry.id).await;
        assert_eq!(failed.state, ProcessingState::Failed);
        assert!(failed.error.unwrap().contains("changed"));
        assert_eq!(failed.error_code, Some(fetch_core::ErrorCode::Conflict));
        let guard = metadata.lock.lock().await;
        let missing_job = manager.retry(retry.id).await.unwrap();
        tokio::fs::remove_file(&file.path).await.unwrap();
        drop(guard);
        assert_eq!(
            wait(&manager, missing_job.id).await.state,
            ProcessingState::Failed
        );
        manager.shutdown().await;
    }
    #[tokio::test]
    async fn restart_removes_only_owned_uncommitted_publications_and_preserves_queued_work() {
        let (dir, storage, file) = fixture().await;
        settings(&storage, dir.path()).await;
        for collision in [false, true] {
            let mut job = ProcessingJob::new(ProcessingKind::Conversion, file.id, "Crash".into());
            job.transition(ProcessingState::Running).unwrap();
            let temp = dir.path().join(format!("{}.part", job.id));
            let output = dir.path().join(format!("{}.mp4", job.id));
            tokio::fs::write(&temp, b"partial").await.unwrap();
            if collision {
                tokio::fs::write(&output, b"unrelated").await.unwrap();
            } else {
                tokio::fs::hard_link(&temp, &output).await.unwrap();
            }
            let work = Work::Export {
                request: export_request(revision(&file.path).await.unwrap()),
                root: dir.path().into(),
                temporary: Some(temp.clone()),
                output: Some(output.clone()),
                cancel_requested: false,
            };
            storage
                .insert_processing_job(&job, &work.json().unwrap())
                .await
                .unwrap();
            let (manager, _) = manager(storage.clone(), missing(dir.path())).await;
            assert_eq!(
                manager
                    .list()
                    .await
                    .unwrap()
                    .into_iter()
                    .find(|j| j.id == job.id)
                    .unwrap()
                    .state,
                ProcessingState::Interrupted
            );
            assert!(!temp.exists());
            assert_eq!(output.exists(), collision);
            manager.shutdown().await;
        }
        let adapter = missing(dir.path());
        let events = EventBus::default();
        let metadata = MetadataService::new(storage.clone(), adapter.media.clone(), events.clone())
            .await
            .unwrap();
        let guard = metadata.lock.lock().await;
        let queued = ProcessingJob::new(ProcessingKind::Conversion, file.id, "Queued".into());
        let work = Work::Export {
            request: export_request(revision(&file.path).await.unwrap()),
            root: dir.path().into(),
            temporary: None,
            output: None,
            cancel_requested: false,
        };
        storage
            .insert_processing_job(&queued, &work.json().unwrap())
            .await
            .unwrap();
        let service = ProcessingManager::new(
            storage,
            metadata.clone(),
            adapter.clone(),
            events,
            adapter.media.shutdown,
        )
        .await
        .unwrap();
        assert_eq!(
            service
                .list()
                .await
                .unwrap()
                .into_iter()
                .find(|j| j.id == queued.id)
                .unwrap()
                .state,
            ProcessingState::Queued
        );
        drop(guard);
        assert_eq!(
            wait(&service, queued.id).await.state,
            ProcessingState::Failed
        );
        service.shutdown().await;
    }
    #[tokio::test]
    #[ignore = "requires installed managed FFmpeg/FFprobe"]
    async fn real_managed_processing_cancel_and_collision_preserve_files() {
        let (dir, storage, file) = fixture().await;
        settings(&storage, dir.path()).await;
        let adapter = real_adapter();
        source(&adapter, &file.path).await;
        let original = tokio::fs::read(&file.path).await.unwrap();
        let (manager, metadata) = manager(storage.clone(), adapter.clone()).await;
        let guard = metadata.lock.lock().await;
        let mut events = manager.inner.events.subscribe();
        let job = manager
            .start_export(file.id, request(&adapter, &file.path).await)
            .await
            .unwrap();
        drop(guard);
        loop {
            if let ApplicationEvent::ProcessingUpdated(event) = events.recv().await.unwrap()
                && event.id == job.id
                && event.state == ProcessingState::Running
            {
                break;
            }
        }
        manager.cancel(job.id).await.unwrap();
        assert_eq!(
            wait(&manager, job.id).await.state,
            ProcessingState::Cancelled
        );
        let guard = metadata.lock.lock().await;
        let job = manager
            .start_export(file.id, request(&adapter, &file.path).await)
            .await
            .unwrap();
        let directory = dir.path().join("Converted");
        tokio::fs::create_dir_all(&directory).await.unwrap();
        let collision = directory.join(format!("Export [{}].mp4", job.id));
        tokio::fs::write(&collision, b"existing output")
            .await
            .unwrap();
        drop(guard);
        assert_eq!(wait(&manager, job.id).await.state, ProcessingState::Failed);
        assert_eq!(
            tokio::fs::read(collision).await.unwrap(),
            b"existing output"
        );
        let mut entries = tokio::fs::read_dir(directory).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            assert!(!entry.file_name().to_string_lossy().starts_with(".fetch-"));
        }
        assert_eq!(tokio::fs::read(&file.path).await.unwrap(), original);
        manager.shutdown().await;
    }

    #[tokio::test]
    #[ignore = "requires installed managed FFmpeg/FFprobe"]
    async fn real_managed_processing_queue_preserves_source_root_and_metadata_history() {
        let (dir, storage, file) = fixture().await;
        settings(&storage, dir.path()).await;
        let adapter = real_adapter();
        source(&adapter, &file.path).await;
        let original = tokio::fs::read(&file.path).await.unwrap();
        let (manager, metadata) = manager(storage.clone(), adapter.clone()).await;
        let guard = metadata.lock.lock().await;
        let r = request(&adapter, &file.path).await;
        let a = manager.start_export(file.id, r.clone()).await.unwrap();
        let b = manager.start_export(file.id, r).await.unwrap();
        let other = dir.path().join("new-root");
        tokio::fs::create_dir(&other).await.unwrap();
        settings(&storage, &other).await;
        drop(guard);
        let first = wait(&manager, a.id).await;
        let second = wait(&manager, b.id).await;
        assert_eq!(first.state, ProcessingState::Completed, "{:?}", first.error);
        assert_eq!(
            second.state,
            ProcessingState::Completed,
            "{:?}",
            second.error
        );
        assert!(second.started_at >= first.finished_at);
        let output = storage
            .get_completed_file(first.output_file_id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(output.path.starts_with(dir.path().join("Converted")));
        assert!(output.job_id.is_none());
        assert_eq!(output.source_file_id, Some(file.id));
        assert!(output.path.exists());
        assert_eq!(tokio::fs::read(&file.path).await.unwrap(), original);
        let m = manager
            .start_metadata(
                file.id,
                MetadataUpdate {
                    revision: revision(&file.path).await.unwrap(),
                    fields: std::collections::BTreeMap::from([("title".into(), "Updated".into())]),
                    artwork: ArtworkUpdate::Keep,
                },
            )
            .await
            .unwrap();
        assert_eq!(
            wait(&manager, m.operation_id.unwrap()).await.state,
            ProcessingState::Completed
        );
        manager.shutdown().await;
        let (next, _) = super::tests::manager(storage.clone(), real_adapter()).await;
        assert_eq!(
            next.metadata_status(file.id).await.unwrap().state,
            MetadataSaveState::Completed
        );
        assert_eq!(next.list().await.unwrap().len(), 3);
        next.shutdown().await;
    }
}
