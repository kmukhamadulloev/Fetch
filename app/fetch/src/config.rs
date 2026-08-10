use std::{
    env,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use serde::Deserialize;
use thiserror::Error;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_LOG_FILTER: &str = "fetch=info,fetch_server=info";

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FetchConfig {
    pub bind_address: IpAddr,
    pub port: u16,
    pub data_directory: PathBuf,
    pub log_filter: String,
    pub download_directory: PathBuf,
    pub concurrent_downloads: usize,
    pub allowed_networks: Vec<String>,
    pub open_browser_on_start: bool,
    pub ytdlp_auto_update: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DEFAULT_PORT,
            data_directory: default_data_directory(),
            log_filter: DEFAULT_LOG_FILTER.to_owned(),
            download_directory: default_download_directory(),
            concurrent_downloads: 3,
            allowed_networks: vec!["192.168.0.0/16".into()],
            open_browser_on_start: true,
            ytdlp_auto_update: true,
        }
    }
}

impl FetchConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let explicit_path = env::var_os("FETCH_CONFIG").map(PathBuf::from);
        let config_path = explicit_path.clone().unwrap_or_else(default_config_path);
        let mut config = if config_path.exists() {
            Self::from_file(&config_path)?
        } else if explicit_path.is_some() {
            return Err(ConfigError::MissingExplicitFile(config_path));
        } else {
            Self::default()
        };

        if let Some(value) = env::var_os("FETCH_BIND_ADDRESS") {
            config.bind_address = value
                .to_string_lossy()
                .parse()
                .map_err(|_| ConfigError::InvalidEnvironment("FETCH_BIND_ADDRESS"))?;
        }
        if let Some(value) = env::var_os("FETCH_PORT") {
            config.port = value
                .to_string_lossy()
                .parse()
                .map_err(|_| ConfigError::InvalidEnvironment("FETCH_PORT"))?;
        }
        if let Some(value) = env::var_os("FETCH_DATA_DIRECTORY") {
            config.data_directory = PathBuf::from(value);
        }
        if let Some(value) = env::var_os("FETCH_LOG") {
            config.log_filter = value.to_string_lossy().into_owned();
        }
        if let Some(value) = env::var_os("FETCH_DOWNLOAD_DIRECTORY") {
            config.download_directory = PathBuf::from(value);
        }
        Ok(config)
    }

    fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_directory.join("data").join("fetch.sqlite3")
    }

    pub fn runtime_path(&self) -> PathBuf {
        self.data_directory.join("runtime")
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("FETCH_CONFIG points to missing file {0}")]
    MissingExplicitFile(PathBuf),
    #[error("could not read configuration file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse configuration file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid value in {0}")]
    InvalidEnvironment(&'static str),
}

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "Fetch", "Fetch")
}

fn default_data_directory() -> PathBuf {
    project_dirs()
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".fetch"))
}

fn default_config_path() -> PathBuf {
    project_dirs()
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from(".fetch/config.toml"))
}

fn default_download_directory() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|path| path.join("Fetch")))
        .unwrap_or_else(|| default_data_directory().join("downloads"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_local_only_and_have_a_database_path() {
        let config = FetchConfig::default();
        assert!(config.bind_address.is_loopback());
        assert_eq!(config.port, 8080);
        assert!(config.database_path().ends_with("data/fetch.sqlite3"));
    }

    #[test]
    fn parses_document_and_rejects_unknown_fields() {
        let config: FetchConfig = toml::from_str(
            r#"
                bind_address = "0.0.0.0"
                port = 9000
                data_directory = "/tmp/fetch-test"
                log_filter = "debug"
                download_directory = "/tmp/fetch-downloads"
                concurrent_downloads = 2
                allowed_networks = ["192.168.0.0/16"]
                open_browser_on_start = false
                ytdlp_auto_update = true
            "#,
        )
        .unwrap();
        assert_eq!(config.bind_address.to_string(), "0.0.0.0");
        assert_eq!(config.port, 9000);

        assert!(toml::from_str::<FetchConfig>("surprise = true").is_err());
    }
}
