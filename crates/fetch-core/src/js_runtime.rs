use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YtDlpJsRuntime {
    #[default]
    Auto,
    Deno,
    Node,
    #[serde(rename = "quickjs")]
    QuickJs,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JavaScriptRuntimeName {
    Deno,
    Node,
    #[serde(rename = "quickjs")]
    QuickJs,
}

impl JavaScriptRuntimeName {
    pub fn executable_name(self) -> &'static str {
        match self {
            Self::Deno => "deno",
            Self::Node => "node",
            Self::QuickJs => "qjs",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaScriptRuntime {
    pub name: JavaScriptRuntimeName,
    pub detected: bool,
    pub version: Option<String>,
}

#[derive(Clone)]
pub struct JsRuntimePolicy {
    current: Arc<RwLock<YtDlpJsRuntime>>,
}

impl JsRuntimePolicy {
    pub fn new(runtime: YtDlpJsRuntime) -> Self {
        Self {
            current: Arc::new(RwLock::new(runtime)),
        }
    }

    pub fn current(&self) -> YtDlpJsRuntime {
        *self
            .current
            .read()
            .expect("JavaScript runtime policy lock is not poisoned")
    }

    pub fn replace(&self, runtime: YtDlpJsRuntime) {
        *self
            .current
            .write()
            .expect("JavaScript runtime policy lock is not poisoned") = runtime;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_is_shared_and_hot_replaceable() {
        let policy = JsRuntimePolicy::new(YtDlpJsRuntime::Auto);
        let consumer = policy.clone();
        policy.replace(YtDlpJsRuntime::Node);
        assert_eq!(consumer.current(), YtDlpJsRuntime::Node);
    }
}
