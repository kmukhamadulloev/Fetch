use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::FetchError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticLogEntry {
    pub id: i64,
    pub level: String,
    pub subsystem: String,
    pub message: String,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub checks: Vec<DiagnosticCheck>,
}

#[async_trait::async_trait]
pub trait DiagnosticOperations: Send + Sync {
    async fn logs(&self) -> Result<Vec<DiagnosticLogEntry>, FetchError>;
    async fn clear_logs(&self) -> Result<(), FetchError>;
    async fn report(&self) -> Result<DiagnosticsReport, FetchError>;
    async fn record_log(
        &self,
        level: &str,
        subsystem: &str,
        message: &str,
        details: Option<&str>,
    ) -> Result<(), FetchError>;
}
