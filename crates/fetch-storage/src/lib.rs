//! SQLite persistence and migrations.

use std::path::{Path, PathBuf};

use fetch_core::{
    ApplicationSettings, CompletedFile, DiagnosticLogEntry, DownloadJob, DownloadStatus,
    FetchError, PlaybackProgress, SettingsOperations,
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
        sqlx::query("INSERT INTO completed_files (id, job_id, filename, path, thumbnail_path, size_bytes, mime_type, title, browser_playable, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(file.id.to_string())
            .bind(file.job_id.to_string())
            .bind(&file.filename)
            .bind(file.path.to_string_lossy().as_ref())
            .bind(file.thumbnail_path.as_ref().map(|path| path.to_string_lossy().into_owned()))
            .bind(i64::try_from(file.size_bytes).map_err(|error| StorageError::Data(error.to_string()))?)
            .bind(&file.mime_type)
            .bind(&file.title)
            .bind(file.browser_playable)
            .bind(file.created_at.to_rfc3339())
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_completed_files(&self) -> Result<Vec<CompletedFile>, StorageError> {
        sqlx::query("SELECT completed_files.*, download_jobs.request_json AS job_request_json, playback_progress.position_seconds AS playback_position_seconds, playback_progress.duration_seconds AS playback_duration_seconds, playback_progress.completed AS playback_completed, playback_progress.updated_at AS playback_updated_at FROM completed_files JOIN download_jobs ON download_jobs.id = completed_files.job_id LEFT JOIN playback_progress ON playback_progress.file_id = completed_files.id ORDER BY completed_files.created_at DESC")
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
        sqlx::query("SELECT completed_files.*, download_jobs.request_json AS job_request_json, playback_progress.position_seconds AS playback_position_seconds, playback_progress.duration_seconds AS playback_duration_seconds, playback_progress.completed AS playback_completed, playback_progress.updated_at AS playback_updated_at FROM completed_files JOIN download_jobs ON download_jobs.id = completed_files.job_id LEFT JOIN playback_progress ON playback_progress.file_id = completed_files.id WHERE completed_files.id = ?")
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
        sqlx::query("SELECT completed_files.*, download_jobs.request_json AS job_request_json, playback_progress.position_seconds AS playback_position_seconds, playback_progress.duration_seconds AS playback_duration_seconds, playback_progress.completed AS playback_completed, playback_progress.updated_at AS playback_updated_at FROM completed_files JOIN download_jobs ON download_jobs.id = completed_files.job_id LEFT JOIN playback_progress ON playback_progress.file_id = completed_files.id WHERE completed_files.job_id = ?")
            .bind(job_id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .map(decode_completed)
            .transpose()
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
    let request: fetch_core::DownloadRequest = from_json(row.try_get("job_request_json")?)?;
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
        job_id: parse(row.try_get::<String, _>("job_id")?)?,
        playlist: request.playlist,
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
    async fn opens_database_and_runs_initial_migration() {
        let storage = Storage::open(Path::new(":memory:")).await.unwrap();
        storage.health_check().await.unwrap();
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(storage.pool())
            .await
            .unwrap();
        assert_eq!(version, 4);
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
            job_id: job.id,
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
}
