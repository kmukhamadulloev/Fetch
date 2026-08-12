use std::sync::Arc;

use fetch_core::{FetchError, ProxyOperations, ProxyPolicy, ProxySettings};
use fetch_storage::Storage;

pub struct ManagedProxy {
    storage: Arc<Storage>,
    policy: ProxyPolicy,
}

impl ManagedProxy {
    pub fn new(storage: Arc<Storage>, policy: ProxyPolicy) -> Self {
        Self { storage, policy }
    }
}

#[async_trait::async_trait]
impl ProxyOperations for ManagedProxy {
    async fn get_proxy(&self) -> Result<ProxySettings, FetchError> {
        Ok(self.policy.current())
    }

    async fn put_proxy(&self, settings: ProxySettings) -> Result<ProxySettings, FetchError> {
        settings.validate()?;
        self.storage
            .save_proxy_settings(&settings)
            .await
            .map_err(|error| FetchError::Internal(error.to_string()))?;
        self.policy.replace(settings.clone())?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fetch_core::ProxyMode;

    #[tokio::test]
    async fn saved_proxy_hot_applies_after_persistence() {
        let storage = Arc::new(
            Storage::open(std::path::Path::new(":memory:"))
                .await
                .unwrap(),
        );
        let policy = ProxyPolicy::new(ProxySettings::default()).unwrap();
        let service = ManagedProxy::new(storage.clone(), policy.clone());
        let settings = ProxySettings {
            mode: ProxyMode::Custom,
            url: Some("http://127.0.0.1:8080".into()),
        };

        service.put_proxy(settings.clone()).await.unwrap();

        assert_eq!(policy.current(), settings);
        assert_eq!(storage.load_proxy_settings().await.unwrap(), settings);
    }
}
