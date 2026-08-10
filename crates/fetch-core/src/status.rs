use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};

/// Public health information returned by the application status service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    pub version: String,
    pub server: ServerStatus,
    pub runtime_ready: bool,
    pub storage_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    Ready,
}

/// Application-layer service used by HTTP transport code.
#[derive(Debug, Clone)]
pub struct StatusService {
    version: String,
    storage_ready: Arc<AtomicBool>,
    runtime_ready: Arc<AtomicBool>,
}

impl StatusService {
    pub fn new(version: impl Into<String>, storage_ready: bool, runtime_ready: bool) -> Self {
        Self {
            version: version.into(),
            storage_ready: Arc::new(AtomicBool::new(storage_ready)),
            runtime_ready: Arc::new(AtomicBool::new(runtime_ready)),
        }
    }

    pub fn status(&self) -> AppStatus {
        AppStatus {
            version: self.version.clone(),
            server: ServerStatus::Ready,
            runtime_ready: self.runtime_ready.load(Ordering::Relaxed),
            storage_ready: self.storage_ready.load(Ordering::Relaxed),
        }
    }

    pub fn set_runtime_ready(&self, ready: bool) {
        self.runtime_ready.store(ready, Ordering::Relaxed);
    }

    pub fn set_storage_ready(&self, ready: bool) {
        self.storage_ready.store(ready, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_service_returns_its_real_dependencies_state() {
        let status = StatusService::new("1.2.3", true, false).status();
        assert_eq!(status.version, "1.2.3");
        assert!(status.storage_ready);
        assert!(!status.runtime_ready);
        assert_eq!(status.server, ServerStatus::Ready);
    }
}
