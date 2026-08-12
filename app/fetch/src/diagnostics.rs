use std::sync::Arc;

use fetch_core::{
    DiagnosticCheck, DiagnosticLogEntry, DiagnosticOperations, DiagnosticsReport, FetchError,
    RuntimeStatus,
};
use fetch_runtime::RuntimeManager;
use fetch_storage::Storage;

pub struct DiagnosticsService {
    storage: Arc<Storage>,
    runtime: RuntimeManager,
}

impl DiagnosticsService {
    pub fn new(storage: Arc<Storage>, runtime: RuntimeManager) -> Self {
        Self { storage, runtime }
    }
}

#[async_trait::async_trait]
impl DiagnosticOperations for DiagnosticsService {
    async fn logs(&self) -> Result<Vec<DiagnosticLogEntry>, FetchError> {
        self.storage
            .list_logs(1000)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))
    }

    async fn clear_logs(&self) -> Result<(), FetchError> {
        self.storage
            .clear_logs()
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))
    }

    async fn report(&self) -> Result<DiagnosticsReport, FetchError> {
        let database = self.storage.health_check().await;
        let settings = self
            .storage
            .load_settings()
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?
            .ok_or(FetchError::NotFound)?;
        let output = check_output_directory(&settings.download_directory).await;
        let mut checks = vec![
            DiagnosticCheck {
                name: "database".into(),
                healthy: database.is_ok(),
                message: database
                    .map(|_| "SQLite is readable".into())
                    .unwrap_or_else(|error| error.to_string()),
            },
            DiagnosticCheck {
                name: "download_directory".into(),
                healthy: output.is_ok(),
                message: output
                    .map(|_| settings.download_directory.display().to_string())
                    .unwrap_or_else(|error| error.to_string()),
            },
        ];
        for component in self.runtime.inspect_all().await {
            checks.push(DiagnosticCheck {
                name: component.name.executable_name().into(),
                healthy: component.status == RuntimeStatus::Ready,
                message: component
                    .version
                    .or(component.error)
                    .unwrap_or_else(|| format!("{:?}", component.status)),
            });
        }
        Ok(DiagnosticsReport { checks })
    }

    async fn record_log(
        &self,
        level: &str,
        subsystem: &str,
        message: &str,
        details: Option<&str>,
    ) -> Result<(), FetchError> {
        self.storage
            .append_log(level, subsystem, message, details)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))
    }
}

async fn check_output_directory(path: &std::path::Path) -> std::io::Result<()> {
    if !tokio::fs::metadata(path).await?.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "configured output path is not a directory",
        ));
    }
    let probe = path.join(format!(".fetch-diagnostic-{}", uuid::Uuid::new_v4()));
    tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
        .await?;
    tokio::fs::remove_file(probe).await
}
