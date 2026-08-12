use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::FetchError;

const MAX_PROXY_URL_LENGTH: usize = 2_048;

#[async_trait::async_trait]
pub trait ProxyOperations: Send + Sync {
    async fn get_proxy(&self) -> Result<ProxySettings, FetchError>;
    async fn put_proxy(&self, settings: ProxySettings) -> Result<ProxySettings, FetchError>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMode {
    #[default]
    System,
    Direct,
    Custom,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxySettings {
    pub mode: ProxyMode,
    pub url: Option<String>,
}

impl std::fmt::Debug for ProxySettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProxySettings")
            .field("mode", &self.mode)
            .field("url", &self.url.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl ProxySettings {
    pub fn validate(&self) -> Result<(), FetchError> {
        match self.mode {
            ProxyMode::System | ProxyMode::Direct => {
                if self.url.is_some() {
                    return Err(FetchError::InvalidSettings(
                        "a proxy URL is allowed only in custom mode".into(),
                    ));
                }
                Ok(())
            }
            ProxyMode::Custom => self.validate_custom_url(),
        }
    }

    fn validate_custom_url(&self) -> Result<(), FetchError> {
        let raw = self.url.as_deref().ok_or_else(|| {
            FetchError::InvalidSettings("custom proxy mode requires a proxy URL".into())
        })?;
        if raw.is_empty() || raw.len() > MAX_PROXY_URL_LENGTH {
            return Err(FetchError::InvalidSettings(
                "the proxy URL has an invalid length".into(),
            ));
        }
        let parsed = url::Url::parse(raw)
            .map_err(|_| FetchError::InvalidSettings("the proxy URL is invalid".into()))?;
        if !matches!(parsed.scheme(), "http" | "https" | "socks4" | "socks5") {
            return Err(FetchError::InvalidSettings(
                "proxy scheme must be http, https, socks4, or socks5".into(),
            ));
        }
        if parsed.host().is_none() {
            return Err(FetchError::InvalidSettings(
                "the proxy URL must include a host".into(),
            ));
        }
        if parsed.port_or_known_default().is_none() {
            return Err(FetchError::InvalidSettings(
                "the proxy URL must include a usable port".into(),
            ));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(FetchError::InvalidSettings(
                "authenticated proxies are not supported in this version".into(),
            ));
        }
        if !matches!(parsed.path(), "" | "/")
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(FetchError::InvalidSettings(
                "the proxy URL cannot include a path, query, or fragment".into(),
            ));
        }
        Ok(())
    }

    pub fn redact(&self, value: &str) -> String {
        let Some(proxy) = self.url.as_deref() else {
            return value.to_owned();
        };
        let redacted = value.replace(proxy, "<redacted-proxy>");
        let without_slash = proxy.trim_end_matches('/');
        if without_slash == proxy {
            redacted
        } else {
            redacted.replace(without_slash, "<redacted-proxy>")
        }
    }
}

#[derive(Clone)]
pub struct ProxyPolicy {
    current: Arc<RwLock<ProxySettings>>,
}

impl ProxyPolicy {
    pub fn new(settings: ProxySettings) -> Result<Self, FetchError> {
        settings.validate()?;
        Ok(Self {
            current: Arc::new(RwLock::new(settings)),
        })
    }

    pub fn current(&self) -> ProxySettings {
        self.current
            .read()
            .expect("proxy policy lock is not poisoned")
            .clone()
    }

    pub fn replace(&self, settings: ProxySettings) -> Result<(), FetchError> {
        settings.validate()?;
        *self
            .current
            .write()
            .expect("proxy policy lock is not poisoned") = settings;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_proxy_modes_without_exposing_values_in_debug() {
        for url in [
            "http://127.0.0.1:8080",
            "https://proxy.example",
            "socks4://127.0.0.1:1080",
            "socks5://[::1]:1080",
        ] {
            let settings = ProxySettings {
                mode: ProxyMode::Custom,
                url: Some(url.into()),
            };
            settings.validate().unwrap();
            assert!(!format!("{settings:?}").contains(url));
        }
        ProxySettings::default().validate().unwrap();
        ProxySettings {
            mode: ProxyMode::Direct,
            url: None,
        }
        .validate()
        .unwrap();
    }

    #[test]
    fn rejects_ambiguous_credentialed_or_unsupported_proxy_settings() {
        for settings in [
            ProxySettings {
                mode: ProxyMode::System,
                url: Some("http://proxy.example".into()),
            },
            ProxySettings {
                mode: ProxyMode::Custom,
                url: None,
            },
            ProxySettings {
                mode: ProxyMode::Custom,
                url: Some("ftp://proxy.example:21".into()),
            },
            ProxySettings {
                mode: ProxyMode::Custom,
                url: Some("http://user:secret@proxy.example:8080".into()),
            },
            ProxySettings {
                mode: ProxyMode::Custom,
                url: Some("socks5://proxy.example".into()),
            },
            ProxySettings {
                mode: ProxyMode::Custom,
                url: Some("http://proxy.example/path?value=secret".into()),
            },
        ] {
            assert!(settings.validate().is_err(), "accepted {settings:?}");
        }
    }

    #[test]
    fn hot_policy_replaces_valid_settings_and_redacts_the_endpoint() {
        let policy = ProxyPolicy::new(ProxySettings::default()).unwrap();
        let custom = ProxySettings {
            mode: ProxyMode::Custom,
            url: Some("http://private.proxy:8080/".into()),
        };
        policy.replace(custom.clone()).unwrap();
        assert_eq!(policy.current(), custom);
        assert_eq!(
            custom.redact("failed through http://private.proxy:8080"),
            "failed through <redacted-proxy>"
        );
    }
}
