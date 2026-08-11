use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::FetchError;

#[async_trait::async_trait]
pub trait SettingsOperations: Send + Sync {
    async fn get_settings(&self) -> Result<ApplicationSettings, FetchError>;
    async fn put_settings(
        &self,
        settings: ApplicationSettings,
    ) -> Result<ApplicationSettings, FetchError>;
}

#[async_trait::async_trait]
pub trait ListenerOperations: Send + Sync {
    async fn rebind(&self, address: SocketAddr) -> Result<bool, FetchError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationSettings {
    pub bind_address: IpAddr,
    pub port: u16,
    pub allowed_networks: Vec<String>,
    pub download_directory: PathBuf,
    pub concurrent_downloads: u8,
    pub open_browser_on_start: bool,
    pub ytdlp_auto_update: bool,
}

impl ApplicationSettings {
    pub fn validate_basic(&self) -> Result<(), FetchError> {
        if self.port == 0 {
            return Err(FetchError::InvalidSettings(
                "port must be between 1 and 65535".into(),
            ));
        }
        if !(1..=16).contains(&self.concurrent_downloads) {
            return Err(FetchError::InvalidSettings(
                "concurrent_downloads must be between 1 and 16".into(),
            ));
        }
        if self.allowed_networks.is_empty() && !self.bind_address.is_loopback() {
            return Err(FetchError::InvalidSettings(
                "at least one allowed network is required for non-loopback binding".into(),
            ));
        }
        for network in &self.allowed_networks {
            network.parse::<ipnet::IpNet>().map_err(|_| {
                FetchError::InvalidSettings(format!("{network} is not a valid CIDR network"))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_concurrency() {
        let settings = ApplicationSettings {
            bind_address: "127.0.0.1".parse().unwrap(),
            port: 8080,
            allowed_networks: vec![],
            download_directory: PathBuf::from("downloads"),
            concurrent_downloads: 0,
            open_browser_on_start: true,
            ytdlp_auto_update: true,
        };
        assert!(settings.validate_basic().is_err());
    }
}
