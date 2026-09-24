//! SQLite persistence and migrations.

use std::path::{Path, PathBuf};

use fetch_core::{
    ApplicationSettings, CompletedFile, DiagnosticLogEntry, DownloadJob, DownloadStatus,
    FetchError, PlaybackProgress, ProxySettings, SettingsOperations, TelegramJobOwner,
    TelegramPendingAction, TelegramRepository, TelegramSettings,
};
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("could not create database directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] sqlx::Error),
    #[error("SQLite migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("stored data is invalid: {0}")]
    Data(String),
}

#[async_trait::async_trait]
impl SettingsOperations for Storage {
    async fn get_settings(&self) -> Result<ApplicationSettings, FetchError> {
        self.load_settings()
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?
            .ok_or(FetchError::NotFound)
    }

    async fn put_settings(
        &self,
        settings: ApplicationSettings,
    ) -> Result<ApplicationSettings, FetchError> {
        settings.validate_basic()?;
        self.save_settings(&settings)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
        Ok(settings)
    }
}

#[async_trait::async_trait]
impl TelegramRepository for Storage {
    async fn load_settings(&self) -> Result<TelegramSettings, FetchError> {
        self.load_telegram_settings().await.map_err(fetch_error)
    }

    async fn save_settings(&self, settings: &TelegramSettings) -> Result<(), FetchError> {
        self.save_telegram_settings(settings)
            .await
            .map_err(fetch_error)
    }

    async fn polling_offset(&self) -> Result<i64, FetchError> {
        self.telegram_polling_offset().await.map_err(fetch_error)
    }

    async fn claim_update(&self, update_id: i64) -> Result<bool, FetchError> {
        self.claim_telegram_update(update_id)
            .await
            .map_err(fetch_error)
    }

    async fn release_update_claim(&self, update_id: i64) -> Result<(), FetchError> {
        self.release_telegram_update_claim(update_id)
            .await
            .map_err(fetch_error)
    }

    async fn complete_update(&self, update_id: i64, next_offset: i64) -> Result<(), FetchError> {
        self.complete_telegram_update(update_id, next_offset)
            .await
            .map_err(fetch_error)
    }

    async fn save_pending_action(&self, action: &TelegramPendingAction) -> Result<(), FetchError> {
        self.save_telegram_pending_action(action)
            .await
            .map_err(fetch_error)
    }

    async fn consume_pending_action(
        &self,
        id: uuid::Uuid,
        user_id: i64,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<TelegramPendingAction>, FetchError> {
        self.consume_telegram_pending_action(id, user_id, now)
            .await
            .map_err(fetch_error)
    }

    async fn associate_job(&self, owner: &TelegramJobOwner) -> Result<(), FetchError> {
        self.associate_telegram_job(owner)
            .await
            .map_err(fetch_error)
    }

    async fn job_owner(&self, job_id: uuid::Uuid) -> Result<Option<TelegramJobOwner>, FetchError> {
        self.telegram_job_owner(job_id).await.map_err(fetch_error)
    }

    async fn owned_job_ids(&self, user_id: i64) -> Result<Vec<uuid::Uuid>, FetchError> {
        self.telegram_owned_job_ids(user_id)
            .await
            .map_err(fetch_error)
    }
}

#[derive(Clone)]
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|source| {
                StorageError::CreateDirectory {
                    path: parent.to_path_buf(),
                    source,
                }
            })?;
        }

        let is_memory = path == Path::new(":memory:");
        let options = if is_memory {
            sqlx::sqlite::SqliteConnectOptions::new().in_memory(true)
        } else {
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
        }
        .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(if is_memory { 1 } else { 5 })
            .connect_with(options)
            .await?;
        sqlx::migrate!().run(&pool).await?;
        // A claim without completion means the previous process stopped before
        // acknowledging the update. It must be retried after restart.
        sqlx::query("DELETE FROM telegram_updates WHERE completed_at IS NULL")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }

    pub async fn health_check(&self) -> Result<(), StorageError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn insert_job(&self, job: &DownloadJob) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO download_jobs (id, request_json, status, progress_json, error_code, error_message, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(job.id.to_string())
        .bind(to_json(&job.request)?)
        .bind(to_json(&job.status)?)
        .bind(to_json(&job.progress)?)
        .bind(&job.error_code)
        .bind(&job.error_message)
        .bind(job.created_at.to_rfc3339())
        .bind(job.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_job(&self, job: &DownloadJob) -> Result<(), StorageError> {
        let result = sqlx::query(
            "UPDATE download_jobs SET request_json = ?, status = ?, progress_json = ?, error_code = ?, error_message = ?, updated_at = ? WHERE id = ?",
        )
        .bind(to_json(&job.request)?)
        .bind(to_json(&job.status)?)
        .bind(to_json(&job.progress)?)
        .bind(&job.error_code)
        .bind(&job.error_message)
        .bind(job.updated_at.to_rfc3339())
        .bind(job.id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(StorageError::Data(format!("job {} does not exist", job.id)));
        }
        Ok(())
    }

    pub async fn get_job(&self, id: uuid::Uuid) -> Result<Option<DownloadJob>, StorageError> {
        let row = sqlx::query("SELECT * FROM download_jobs WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(decode_job).transpose()
    }

    pub async fn list_jobs(&self) -> Result<Vec<DownloadJob>, StorageError> {
        sqlx::query("SELECT * FROM download_jobs ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(decode_job)
            .collect()
    }

    pub async fn recover_interrupted_jobs(&self) -> Result<u64, StorageError> {
        let stopped = to_json(&DownloadStatus::Stopped)?;
        let result = sqlx::query(
            "UPDATE download_jobs SET status = ?, error_code = 'PROCESS_INTERRUPTED', error_message = 'Fetch stopped while this job was active', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE status IN (?, ?, ?)",
        )
        .bind(stopped)
        .bind(to_json(&DownloadStatus::Queued)?)
        .bind(to_json(&DownloadStatus::Downloading)?)
        .bind(to_json(&DownloadStatus::Postprocessing)?)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_job(&self, id: uuid::Uuid) -> Result<bool, StorageError> {
        Ok(sqlx::query("DELETE FROM download_jobs WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    pub async fn insert_completed_file(&self, file: &CompletedFile) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        Self::insert_completed_in(&mut transaction, file).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn insert_completed_in(
        transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        file: &CompletedFile,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO completed_files (id, job_id, filename, path, thumbnail_path, size_bytes, mime_type, title, browser_playable, created_at, origin, source_file_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(file.id.to_string())
            .bind(file.job_id.map(|id| id.to_string()))
            .bind(&file.filename)
            .bind(file.path.to_string_lossy().as_ref())
            .bind(file.thumbnail_path.as_ref().map(|path| path.to_string_lossy().into_owned()))
            .bind(i64::try_from(file.size_bytes).map_err(|error| StorageError::Data(error.to_string()))?)
            .bind(&file.mime_type)
            .bind(&file.title)
            .bind(file.browser_playable)
            .bind(file.created_at.to_rfc3339())
            .bind(file.origin.map(|kind| match kind { fetch_core::ProcessingKind::Conversion => "conversion", fetch_core::ProcessingKind::Edit => "edit", fetch_core::ProcessingKind::Metadata => "metadata" }))
            .bind(file.source_file_id.map(|id| id.to_string()))
            .execute(&mut **transaction).await?;
        Ok(())
    }

    pub async fn insert_processing_job(
        &self,
        job: &fetch_core::ProcessingJob,
        private_json: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO processing_jobs (id, record_json, private_json, created_at) VALUES (?, ?, ?, ?)")
            .bind(job.id.to_string()).bind(to_json(job)?).bind(private_json).bind(job.created_at.to_rfc3339())
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_processing_job(
        &self,
        job: &fetch_core::ProcessingJob,
        private_json: &str,
    ) -> Result<(), StorageError> {
        let result = sqlx::query(
            "UPDATE processing_jobs SET record_json = ?, private_json = ? WHERE id = ?",
        )
        .bind(to_json(job)?)
        .bind(private_json)
        .bind(job.id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() != 1 {
            return Err(StorageError::Data("Processing job not found".into()));
        }
        Ok(())
    }

    pub async fn processing_jobs(
        &self,
    ) -> Result<Vec<(fetch_core::ProcessingJob, String)>, StorageError> {
        sqlx::query("SELECT record_json, private_json FROM processing_jobs ORDER BY created_at ASC, rowid ASC")
            .fetch_all(&self.pool).await?.into_iter().map(|row| Ok((from_json(row.try_get("record_json")?)?, row.try_get("private_json")?))).collect()
    }

    /// Commit the library entry and terminal process state in the same transaction.
    pub async fn finish_processing_job(
        &self,
        job: &fetch_core::ProcessingJob,
        private_json: &str,
        file: &CompletedFile,
    ) -> Result<(), StorageError> {
        if job.state != fetch_core::ProcessingState::Completed
            || job.output_file_id != Some(file.id)
            || file.source_file_id != Some(job.source_file_id)
            || file.origin != Some(job.kind)
        {
            return Err(StorageError::Data(
                "Invalid completed processing output".into(),
            ));
        }
        let mut transaction = self.pool.begin().await?;
        Self::insert_completed_in(&mut transaction, file).await?;
        let result = sqlx::query(
            "UPDATE processing_jobs SET record_json = ?, private_json = ? WHERE id = ?",
        )
        .bind(to_json(job)?)
        .bind(private_json)
        .bind(job.id.to_string())
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(StorageError::Data("Processing job not found".into()));
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_completed_files(&self) -> Result<Vec<CompletedFile>, StorageError> {
        sqlx::query("SELECT completed_files.*, download_jobs.request_json AS job_request_json, playback_progress.position_seconds AS playback_position_seconds, playback_progress.duration_seconds AS playback_duration_seconds, playback_progress.completed AS playback_completed, playback_progress.updated_at AS playback_updated_at FROM completed_files LEFT JOIN download_jobs ON download_jobs.id = completed_files.job_id LEFT JOIN playback_progress ON playback_progress.file_id = completed_files.id ORDER BY completed_files.created_at DESC")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(decode_completed)
            .collect()
    }

    pub async fn get_completed_file(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<CompletedFile>, StorageError> {
        sqlx::query("SELECT completed_files.*, download_jobs.request_json AS job_request_json, playback_progress.position_seconds AS playback_position_seconds, playback_progress.duration_seconds AS playback_duration_seconds, playback_progress.completed AS playback_completed, playback_progress.updated_at AS playback_updated_at FROM completed_files LEFT JOIN download_jobs ON download_jobs.id = completed_files.job_id LEFT JOIN playback_progress ON playback_progress.file_id = completed_files.id WHERE completed_files.id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .map(decode_completed)
            .transpose()
    }

    pub async fn get_completed_file_for_job(
        &self,
        job_id: uuid::Uuid,
    ) -> Result<Option<CompletedFile>, StorageError> {
        sqlx::query("SELECT completed_files.*, download_jobs.request_json AS job_request_json, playback_progress.position_seconds AS playback_position_seconds, playback_progress.duration_seconds AS playback_duration_seconds, playback_progress.completed AS playback_completed, playback_progress.updated_at AS playback_updated_at FROM completed_files LEFT JOIN download_jobs ON download_jobs.id = completed_files.job_id LEFT JOIN playback_progress ON playback_progress.file_id = completed_files.id WHERE completed_files.job_id = ?")
            .bind(job_id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .map(decode_completed)
            .transpose()
    }

    pub async fn begin_metadata_edit(
        &self,
        id: uuid::Uuid,
        journal: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO metadata_edits (file_id, journal_json) VALUES (?, ?)")
            .bind(id.to_string())
            .bind(journal)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn metadata_edits(&self) -> Result<Vec<(uuid::Uuid, String, bool)>, StorageError> {
        let rows = sqlx::query("SELECT file_id, journal_json, committed FROM metadata_edits")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok((
                    row.try_get::<String, _>("file_id")?
                        .parse()
                        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?,
                    row.try_get("journal_json")?,
                    row.try_get("committed")?,
                ))
            })
            .collect()
    }

    pub async fn finish_metadata_edit(&self, file: &CompletedFile) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "UPDATE completed_files SET title = ?, size_bytes = ?, thumbnail_path = ? WHERE id = ?",
        )
        .bind(&file.title)
        .bind(file.size_bytes as i64)
        .bind(
            file.thumbnail_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        )
        .bind(file.id.to_string())
        .execute(&mut *transaction)
        .await?;
        sqlx::query("UPDATE metadata_edits SET committed = 1 WHERE file_id = ?")
            .bind(file.id.to_string())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn clear_metadata_edit(&self, id: uuid::Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM metadata_edits WHERE file_id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_completed_file(&self, id: uuid::Uuid) -> Result<bool, StorageError> {
        Ok(sqlx::query("DELETE FROM completed_files WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    pub async fn save_playback_progress(
        &self,
        progress: &PlaybackProgress,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO playback_progress (file_id, position_seconds, duration_seconds, completed, updated_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(file_id) DO UPDATE SET position_seconds = excluded.position_seconds, duration_seconds = excluded.duration_seconds, completed = excluded.completed, updated_at = excluded.updated_at")
            .bind(progress.file_id.to_string())
            .bind(progress.position_seconds)
            .bind(progress.duration_seconds)
            .bind(progress.completed)
            .bind(progress.updated_at.to_rfc3339())
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_playback_progress(&self, id: uuid::Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM playback_progress WHERE file_id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn load_settings(&self) -> Result<Option<ApplicationSettings>, StorageError> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'application'")
                .fetch_optional(&self.pool)
                .await?;
        value
            .map(|value| {
                serde_json::from_str(&value).map_err(|error| StorageError::Data(error.to_string()))
            })
            .transpose()
    }

    pub async fn save_settings(&self, settings: &ApplicationSettings) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('application', ?, CURRENT_TIMESTAMP) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP")
            .bind(to_json(settings)?).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn load_proxy_settings(&self) -> Result<ProxySettings, StorageError> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'proxy'")
                .fetch_optional(&self.pool)
                .await?;
        let settings: ProxySettings = value
            .map(from_json)
            .transpose()
            .map(|settings| settings.unwrap_or_default())?;
        settings
            .validate()
            .map_err(|error| StorageError::Data(error.public_message()))?;
        Ok(settings)
    }

    pub async fn save_proxy_settings(&self, settings: &ProxySettings) -> Result<(), StorageError> {
        settings
            .validate()
            .map_err(|error| StorageError::Data(error.public_message()))?;
        sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('proxy', ?, CURRENT_TIMESTAMP) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP")
            .bind(to_json(settings)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn load_telegram_settings(&self) -> Result<TelegramSettings, StorageError> {
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'telegram'")
                .fetch_optional(&self.pool)
                .await?;
        let settings: TelegramSettings = value.map(from_json).transpose()?.unwrap_or_default();
        settings
            .validate()
            .map_err(|error| StorageError::Data(error.public_message()))?;
        Ok(settings)
    }

    pub async fn save_telegram_settings(
        &self,
        settings: &TelegramSettings,
    ) -> Result<(), StorageError> {
        settings
            .validate()
            .map_err(|error| StorageError::Data(error.public_message()))?;
        sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('telegram', ?, CURRENT_TIMESTAMP) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP")
            .bind(to_json(settings)?)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn telegram_polling_offset(&self) -> Result<i64, StorageError> {
        sqlx::query_scalar("SELECT polling_offset FROM telegram_state WHERE singleton = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(StorageError::from)
    }

    pub async fn claim_telegram_update(&self, update_id: i64) -> Result<bool, StorageError> {
        if update_id < 0 {
            return Err(StorageError::Data(
                "Telegram update IDs must not be negative".into(),
            ));
        }
        let result = sqlx::query(
            "INSERT OR IGNORE INTO telegram_updates (update_id, claimed_at) VALUES (?, ?)",
        )
        .bind(update_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn release_telegram_update_claim(&self, update_id: i64) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM telegram_updates WHERE update_id = ?")
            .bind(update_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn complete_telegram_update(
        &self,
        update_id: i64,
        next_offset: i64,
    ) -> Result<(), StorageError> {
        if update_id < 0 || next_offset <= update_id {
            return Err(StorageError::Data(
                "Telegram completion requires a valid increasing offset".into(),
            ));
        }
        let mut transaction = self.pool.begin().await?;
        let completed_at = chrono::Utc::now().to_rfc3339();
        let result =
            sqlx::query("UPDATE telegram_updates SET completed_at = ? WHERE update_id = ?")
                .bind(&completed_at)
                .bind(update_id)
                .execute(&mut *transaction)
                .await?;
        if result.rows_affected() == 0 {
            return Err(StorageError::Data(format!(
                "Telegram update {update_id} was not claimed"
            )));
        }
        sqlx::query("UPDATE telegram_state SET polling_offset = MAX(polling_offset, ?), updated_at = CURRENT_TIMESTAMP WHERE singleton = 1")
            .bind(next_offset)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "DELETE FROM telegram_updates WHERE completed_at IS NOT NULL AND update_id < ?",
        )
        .bind(next_offset.saturating_sub(10_000))
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn save_telegram_pending_action(
        &self,
        action: &TelegramPendingAction,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO telegram_pending_actions (id, user_id, action_json, expires_at, consumed_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET user_id = excluded.user_id, action_json = excluded.action_json, expires_at = excluded.expires_at, consumed_at = excluded.consumed_at")
            .bind(action.id.to_string())
            .bind(action.user_id)
            .bind(to_json(action)?)
            .bind(action.expires_at.to_rfc3339())
            .bind(action.consumed_at.map(|value| value.to_rfc3339()))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn consume_telegram_pending_action(
        &self,
        id: uuid::Uuid,
        user_id: i64,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<TelegramPendingAction>, StorageError> {
        let value: Option<String> = sqlx::query_scalar("UPDATE telegram_pending_actions SET consumed_at = ? WHERE id = ? AND user_id = ? AND consumed_at IS NULL AND expires_at > ? RETURNING action_json")
            .bind(now.to_rfc3339())
            .bind(id.to_string())
            .bind(user_id)
            .bind(now.to_rfc3339())
            .fetch_optional(&self.pool)
            .await?;
        value
            .map(|value| {
                let mut action: TelegramPendingAction = from_json(value)?;
                action.consumed_at = Some(now);
                Ok(action)
            })
            .transpose()
    }

    pub async fn associate_telegram_job(
        &self,
        owner: &TelegramJobOwner,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO telegram_job_owners (job_id, user_id, chat_id) VALUES (?, ?, ?) ON CONFLICT(job_id) DO UPDATE SET user_id = excluded.user_id, chat_id = excluded.chat_id")
            .bind(owner.job_id.to_string())
            .bind(owner.user_id)
            .bind(owner.chat_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn telegram_job_owner(
        &self,
        job_id: uuid::Uuid,
    ) -> Result<Option<TelegramJobOwner>, StorageError> {
        sqlx::query("SELECT job_id, user_id, chat_id FROM telegram_job_owners WHERE job_id = ?")
            .bind(job_id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .map(decode_telegram_job_owner)
            .transpose()
    }

    pub async fn telegram_owned_job_ids(
        &self,
        user_id: i64,
    ) -> Result<Vec<uuid::Uuid>, StorageError> {
        sqlx::query_scalar::<_, String>(
            "SELECT job_id FROM telegram_job_owners WHERE user_id = ? ORDER BY rowid DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(parse)
        .collect()
    }

    pub async fn append_log(
        &self,
        level: &str,
        subsystem: &str,
        message: &str,
        details: Option<&str>,
    ) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO diagnostic_logs (level, subsystem, message, details, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(level).bind(subsystem).bind(message).bind(details).bind(chrono::Utc::now().to_rfc3339())
            .execute(&self.pool).await?;
        sqlx::query("DELETE FROM diagnostic_logs WHERE id NOT IN (SELECT id FROM diagnostic_logs ORDER BY id DESC LIMIT 5000)")
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_logs(&self, limit: u32) -> Result<Vec<DiagnosticLogEntry>, StorageError> {
        sqlx::query("SELECT * FROM diagnostic_logs ORDER BY id DESC LIMIT ?")
            .bind(i64::from(limit.min(1000)))
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| {
                Ok(DiagnosticLogEntry {
                    id: row.try_get("id")?,
                    level: row.try_get("level")?,
                    subsystem: row.try_get("subsystem")?,
                    message: row.try_get("message")?,
                    details: row.try_get("details")?,
                    created_at: parse(row.try_get::<String, _>("created_at")?)?,
                })
            })
            .collect()
    }

    pub async fn clear_logs(&self) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM diagnostic_logs")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_metadata(&self, key: &str) -> Result<Option<String>, StorageError> {
        sqlx::query_scalar("SELECT value FROM app_metadata WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(StorageError::from)
    }

    pub async fn set_metadata(&self, key: &str, value: &str) -> Result<(), StorageError> {
        sqlx::query("INSERT INTO app_metadata (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP")
            .bind(key).bind(value).execute(&self.pool).await?;
        Ok(())
    }
}

fn to_json(value: &impl serde::Serialize) -> Result<String, StorageError> {
    serde_json::to_string(value).map_err(|error| StorageError::Data(error.to_string()))
}

fn decode_job(row: sqlx::sqlite::SqliteRow) -> Result<DownloadJob, StorageError> {
    Ok(DownloadJob {
        id: parse(row.try_get::<String, _>("id")?)?,
        request: from_json(row.try_get("request_json")?)?,
        status: from_json(row.try_get("status")?)?,
        progress: from_json(row.try_get("progress_json")?)?,
        error_code: row.try_get("error_code")?,
        error_message: row.try_get("error_message")?,
        created_at: parse(row.try_get::<String, _>("created_at")?)?,
        updated_at: parse(row.try_get::<String, _>("updated_at")?)?,
    })
}

fn decode_completed(row: sqlx::sqlite::SqliteRow) -> Result<CompletedFile, StorageError> {
    let size: i64 = row.try_get("size_bytes")?;
    let request = row
        .try_get::<Option<String>, _>("job_request_json")?
        .map(from_json::<fetch_core::DownloadRequest>)
        .transpose()?;
    let thumbnail_path = row
        .try_get::<Option<String>, _>("thumbnail_path")?
        .map(PathBuf::from);
    let playback = row
        .try_get::<Option<f64>, _>("playback_position_seconds")?
        .map(
            |position_seconds| -> Result<PlaybackProgress, StorageError> {
                Ok(PlaybackProgress {
                    file_id: parse(row.try_get::<String, _>("id")?)?,
                    position_seconds,
                    duration_seconds: row.try_get("playback_duration_seconds")?,
                    completed: row.try_get("playback_completed")?,
                    updated_at: parse(row.try_get::<String, _>("playback_updated_at")?)?,
                })
            },
        )
        .transpose()?;
    Ok(CompletedFile {
        id: parse(row.try_get::<String, _>("id")?)?,
        job_id: row
            .try_get::<Option<String>, _>("job_id")?
            .map(parse)
            .transpose()?,
        origin: row
            .try_get::<Option<String>, _>("origin")?
            .map(|value| from_json(format!("\"{value}\"")))
            .transpose()?,
        source_file_id: row
            .try_get::<Option<String>, _>("source_file_id")?
            .map(parse)
            .transpose()?,
        playlist: request.and_then(|r| r.playlist),
        filename: row.try_get("filename")?,
        path: PathBuf::from(row.try_get::<String, _>("path")?),
        thumbnail_available: thumbnail_path.is_some(),
        thumbnail_path,
        size_bytes: size
            .try_into()
            .map_err(|error: std::num::TryFromIntError| StorageError::Data(error.to_string()))?,
        mime_type: row.try_get("mime_type")?,
        title: row.try_get("title")?,
        browser_playable: row.try_get("browser_playable")?,
        playback,
        created_at: parse(row.try_get::<String, _>("created_at")?)?,
    })
}

fn decode_telegram_job_owner(
    row: sqlx::sqlite::SqliteRow,
) -> Result<TelegramJobOwner, StorageError> {
    Ok(TelegramJobOwner {
        job_id: parse(row.try_get("job_id")?)?,
        user_id: row.try_get("user_id")?,
        chat_id: row.try_get("chat_id")?,
    })
}

fn fetch_error(error: StorageError) -> FetchError {
    FetchError::Internal(error.to_string())
}

fn from_json<T: serde::de::DeserializeOwned>(value: String) -> Result<T, StorageError> {
    serde_json::from_str(&value).map_err(|error| StorageError::Data(error.to_string()))
}

fn parse<T>(value: String) -> Result<T, StorageError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|error: T::Err| StorageError::Data(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn processing_history_and_private_recovery_options_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("processing.sqlite");
        let storage = Storage::open(&path).await.unwrap();
        let mut job = fetch_core::ProcessingJob::new(
            fetch_core::ProcessingKind::Edit,
            uuid::Uuid::new_v4(),
            "Edited copy".into(),
        );
        let private = r#"{"root":"/private/Edits","revision":"42-1000","options":{"rotate":90}}"#;
        storage.insert_processing_job(&job, private).await.unwrap();
        assert!(storage.insert_processing_job(&job, private).await.is_err());
        job.transition(fetch_core::ProcessingState::Running)
            .unwrap();
        job.progress_percent = Some(42.);
        storage.update_processing_job(&job, private).await.unwrap();
        storage.pool.close().await;
        let storage = Storage::open(&path).await.unwrap();
        let records = storage.processing_jobs().await.unwrap();
        assert_eq!(records, vec![(job.clone(), private.into())]);
        // Recovery must be owned by the application, not fabricated by storage.
        assert_eq!(records[0].0.state, fetch_core::ProcessingState::Running);
        assert!(
            !serde_json::to_string(&records[0].0)
                .unwrap()
                .contains("/private")
        );
        job.id = uuid::Uuid::new_v4();
        assert!(storage.update_processing_job(&job, private).await.is_err());
    }

    #[tokio::test]
    async fn processing_migration_preserves_library_dependents_and_accepts_independent_exports() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .in_memory(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        for migration in sqlx::migrate!().iter().filter(|m| m.version < 8) {
            sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
        }
        let source = uuid::Uuid::new_v4();
        let download = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO download_jobs VALUES (?, '{}', 'completed', '{}', NULL, NULL, '2026-09-23T00:00:00Z', '2026-09-23T00:00:00Z')").bind(download.to_string()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO completed_files (id,job_id,filename,path,size_bytes,mime_type,title,browser_playable,created_at,thumbnail_path) VALUES (?,?,'original.mp4','/media/original.mp4',42,'video/mp4','Original',1,'2026-09-23T00:00:00Z','/media/cover.jpg')").bind(source.to_string()).bind(download.to_string()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO playback_progress VALUES (?,12,42,0,'2026-09-23T00:00:00Z')")
            .bind(source.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO metadata_edits VALUES (?, '{\"source\":\"original\"}', 0)")
            .bind(source.to_string())
            .execute(&pool)
            .await
            .unwrap();
        let mut tx = pool.begin().await.unwrap();
        sqlx::raw_sql(include_str!("../migrations/0008_processing.sql"))
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let preserved: (String, String, String) =
            sqlx::query_as("SELECT job_id,path,thumbnail_path FROM completed_files WHERE id=?")
                .bind(source.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            preserved,
            (
                download.to_string(),
                "/media/original.mp4".into(),
                "/media/cover.jpg".into()
            )
        );
        let position: f64 = sqlx::query_scalar("SELECT position_seconds FROM playback_progress")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(position, 12.);
        let journal: String = sqlx::query_scalar("SELECT journal_json FROM metadata_edits")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(journal, r#"{"source":"original"}"#);
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&pool)
                .await
                .unwrap()
                .is_empty()
        );
        let storage = Storage { pool };
        let now = chrono::Utc::now();
        let file = CompletedFile {
            id: uuid::Uuid::new_v4(),
            job_id: None,
            origin: Some(fetch_core::ProcessingKind::Conversion),
            source_file_id: Some(source),
            playlist: None,
            filename: "converted.mp4".into(),
            path: "/media/Converted/converted.mp4".into(),
            thumbnail_path: None,
            thumbnail_available: false,
            size_bytes: 20,
            mime_type: "video/mp4".into(),
            title: Some("Converted".into()),
            browser_playable: true,
            playback: None,
            created_at: now,
        };
        let mut job = fetch_core::ProcessingJob {
            id: uuid::Uuid::new_v4(),
            kind: fetch_core::ProcessingKind::Conversion,
            source_file_id: source,
            output_file_id: None,
            title: "Converted".into(),
            state: fetch_core::ProcessingState::Queued,
            stage: "queued".into(),
            progress_percent: None,
            eta_seconds: None,
            created_at: now,
            started_at: None,
            updated_at: now,
            finished_at: None,
            error: None,
            error_code: None,
        };
        storage
            .insert_processing_job(&job, r#"{"root":"/media"}"#)
            .await
            .unwrap();
        job.state = fetch_core::ProcessingState::Completed;
        job.output_file_id = Some(file.id);
        storage
            .finish_processing_job(&job, "{}", &file)
            .await
            .unwrap();
        let loaded = storage.get_completed_file(file.id).await.unwrap().unwrap();
        assert_eq!(loaded, file);
        assert_eq!(storage.processing_jobs().await.unwrap()[0].0, job);
        storage.clear_metadata_edit(source).await.unwrap();
        storage.delete_completed_file(source).await.unwrap();
        assert_eq!(
            storage
                .get_completed_file(file.id)
                .await
                .unwrap()
                .unwrap()
                .source_file_id,
            Some(source)
        );
        assert!(
            sqlx::query("SELECT * FROM playback_progress")
                .fetch_all(&storage.pool)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&storage.pool)
                .await
                .unwrap()
                .is_empty()
        );
        let mut failed_output = file.clone();
        failed_output.id = uuid::Uuid::new_v4();
        failed_output.path = "/media/Converted/rollback.mp4".into();
        job.id = uuid::Uuid::new_v4();
        job.output_file_id = Some(failed_output.id);
        assert!(
            storage
                .finish_processing_job(&job, "{}", &failed_output)
                .await
                .is_err()
        );
        assert!(
            storage
                .get_completed_file(failed_output.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn opens_database_and_runs_initial_migration() {
        let storage = Storage::open(Path::new(":memory:")).await.unwrap();
        storage.health_check().await.unwrap();
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(storage.pool())
            .await
            .unwrap();
        assert_eq!(version, 8);
        assert_eq!(
            storage.load_proxy_settings().await.unwrap(),
            ProxySettings::default()
        );
    }

    #[tokio::test]
    async fn persists_proxy_settings_separately_from_application_settings() {
        let storage = Storage::open(Path::new(":memory:")).await.unwrap();
        let settings = ProxySettings {
            mode: fetch_core::ProxyMode::Custom,
            url: Some("socks5://127.0.0.1:1080".into()),
        };
        storage.save_proxy_settings(&settings).await.unwrap();
        assert_eq!(storage.load_proxy_settings().await.unwrap(), settings);
        assert!(storage.load_settings().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn persists_jobs_and_recovers_interrupted_state() {
        let storage = Storage::open(Path::new(":memory:")).await.unwrap();
        let mut job = DownloadJob::new(fetch_core::DownloadRequest {
            url: "https://example.test/media".into(),
            title: Some("Example".into()),
            duration_seconds: Some(42.0),
            mode: fetch_core::DownloadMode::Video,
            format_id: None,
            quality: None,
            container: None,
            video_codec: None,
            audio_codec: None,
            embed_metadata: false,
            embed_thumbnail: false,
            subtitles: false,
            playlist: Some(fetch_core::PlaylistContext {
                id: "fixture-playlist".into(),
                title: "Fixture playlist".into(),
                index: 2,
            }),
            output_directory: Some(PathBuf::from("/tmp")),
        });
        job.transition(DownloadStatus::Queued).unwrap();
        storage.insert_job(&job).await.unwrap();
        assert_eq!(storage.recover_interrupted_jobs().await.unwrap(), 1);
        let loaded = storage.get_job(job.id).await.unwrap().unwrap();
        assert_eq!(loaded.status, DownloadStatus::Stopped);
        assert_eq!(loaded.request.title.as_deref(), Some("Example"));

        let completed = CompletedFile {
            id: uuid::Uuid::new_v4(),
            job_id: Some(job.id),
            origin: None,
            source_file_id: None,
            playlist: job.request.playlist.clone(),
            filename: "media.mp4".into(),
            path: PathBuf::from("/tmp/media.mp4"),
            thumbnail_path: Some(PathBuf::from("/data/thumbnails/job.jpg")),
            thumbnail_available: true,
            size_bytes: 42,
            mime_type: "video/mp4".into(),
            title: Some("Example".into()),
            browser_playable: true,
            playback: None,
            created_at: chrono::Utc::now(),
        };
        storage.insert_completed_file(&completed).await.unwrap();
        let loaded = storage
            .get_completed_file_for_job(job.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.thumbnail_path, completed.thumbnail_path);
        assert!(loaded.thumbnail_available);
        assert_eq!(loaded.playlist, completed.playlist);

        let progress = PlaybackProgress {
            file_id: completed.id,
            position_seconds: 21.0,
            duration_seconds: 42.0,
            completed: false,
            updated_at: chrono::Utc::now(),
        };
        storage.save_playback_progress(&progress).await.unwrap();
        let loaded = storage
            .get_completed_file(completed.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.playback, Some(progress));
    }

    #[tokio::test]
    async fn persists_telegram_settings_without_a_token_and_deduplicates_updates() {
        let storage = Storage::open(Path::new(":memory:")).await.unwrap();
        assert_eq!(
            storage.load_telegram_settings().await.unwrap(),
            TelegramSettings::default()
        );

        let settings = TelegramSettings {
            enabled: true,
            use_proxy: true,
            send_completed_media: true,
            upload_limit_mb: 25,
            allowed_user_ids: vec![42],
            privacy_acknowledged: true,
            ..TelegramSettings::default()
        };
        storage.save_telegram_settings(&settings).await.unwrap();
        assert_eq!(storage.load_telegram_settings().await.unwrap(), settings);
        let serialized: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'telegram'")
                .fetch_one(storage.pool())
                .await
                .unwrap();
        assert!(!serialized.contains("token"));

        assert!(storage.claim_telegram_update(100).await.unwrap());
        assert!(!storage.claim_telegram_update(100).await.unwrap());
        storage.release_telegram_update_claim(100).await.unwrap();
        assert!(storage.claim_telegram_update(100).await.unwrap());
        storage.complete_telegram_update(100, 101).await.unwrap();
        assert!(!storage.claim_telegram_update(100).await.unwrap());
        storage.claim_telegram_update(200).await.unwrap();
        assert!(storage.complete_telegram_update(200, 200).await.is_err());
        assert_eq!(storage.telegram_polling_offset().await.unwrap(), 101);
    }

    #[tokio::test]
    async fn retries_incomplete_telegram_claims_after_process_restart() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fetch.sqlite3");
        let storage = Storage::open(&path).await.unwrap();
        assert!(storage.claim_telegram_update(42).await.unwrap());
        storage.pool.close().await;

        let storage = Storage::open(&path).await.unwrap();
        assert!(storage.claim_telegram_update(42).await.unwrap());
        storage.complete_telegram_update(42, 43).await.unwrap();
        storage.pool.close().await;

        let storage = Storage::open(&path).await.unwrap();
        assert!(!storage.claim_telegram_update(42).await.unwrap());
        assert_eq!(storage.telegram_polling_offset().await.unwrap(), 43);
    }

    #[tokio::test]
    async fn atomically_consumes_owned_pending_actions_once() {
        let storage = Storage::open(Path::new(":memory:")).await.unwrap();
        let now = chrono::Utc::now();
        let action = TelegramPendingAction {
            id: uuid::Uuid::new_v4(),
            user_id: 42,
            chat_id: 42,
            source_url: "https://example.test/media".into(),
            media: fetch_core::MediaInfo {
                kind: fetch_core::MediaKind::Media,
                id: Some("fixture".into()),
                extractor: Some("fixture".into()),
                title: "Fixture".into(),
                webpage_url: Some("https://example.test/media".into()),
                duration_seconds: Some(42.0),
                thumbnail_url: None,
                playlist_count: None,
                entries: Vec::new(),
                formats: Vec::new(),
            },
            stage: fetch_core::TelegramPendingStage::ChooseMode,
            mode: None,
            expires_at: now + chrono::Duration::minutes(5),
            consumed_at: None,
        };
        storage.save_telegram_pending_action(&action).await.unwrap();

        assert!(
            storage
                .consume_telegram_pending_action(action.id, 7, now)
                .await
                .unwrap()
                .is_none()
        );
        let consumed = storage
            .consume_telegram_pending_action(action.id, 42, now)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(consumed.consumed_at, Some(now));
        assert!(
            storage
                .consume_telegram_pending_action(action.id, 42, now)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn correlates_telegram_jobs_with_their_owner() {
        let storage = Storage::open(Path::new(":memory:")).await.unwrap();
        let job = DownloadJob::new(fetch_core::DownloadRequest {
            url: "https://example.test/media".into(),
            title: None,
            duration_seconds: None,
            mode: fetch_core::DownloadMode::Video,
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
        });
        storage.insert_job(&job).await.unwrap();
        let owner = TelegramJobOwner {
            job_id: job.id,
            user_id: 42,
            chat_id: 42,
        };
        storage.associate_telegram_job(&owner).await.unwrap();

        assert_eq!(
            storage.telegram_job_owner(job.id).await.unwrap(),
            Some(owner)
        );
        assert_eq!(
            storage.telegram_owned_job_ids(42).await.unwrap(),
            vec![job.id]
        );
        assert!(storage.telegram_owned_job_ids(7).await.unwrap().is_empty());
    }
}
