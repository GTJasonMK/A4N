use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceMode {
    Embedded,
    Sidecar,
    Remote,
    Disabled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestartPolicy {
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,
}

impl RestartPolicy {
    pub fn backoff_ms(self, restart_count: u32) -> u64 {
        let shift = restart_count.saturating_sub(1).min(31);
        self.initial_backoff_ms
            .saturating_mul(1_u64 << shift)
            .min(self.max_backoff_ms)
    }
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: default_max_restarts(),
            initial_backoff_ms: default_initial_backoff_ms(),
            max_backoff_ms: default_max_backoff_ms(),
        }
    }
}

/// 远程 sidecar 的独立部署凭证与 TLS 身份配置。
/// 只保存环境变量名和证书路径，不把 token/private key 内容放进配置合同。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteSecurityConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_cert_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_cert_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_key_path: Option<PathBuf>,
}

impl RemoteSecurityConfig {
    pub fn is_empty(&self) -> bool {
        self.auth_token_env.is_none()
            && self.ca_cert_path.is_none()
            && self.client_cert_path.is_none()
            && self.client_key_path.is_none()
    }

    pub fn has_client_identity(&self) -> bool {
        self.client_cert_path.is_some() && self.client_key_path.is_some()
    }

    pub fn has_deployment_credential(&self) -> bool {
        self.auth_token_env.is_some() || self.has_client_identity()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedServiceConfig {
    pub name: String,
    pub mode: ServiceMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default = "default_health_path")]
    pub health_path: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default = "default_startup_timeout_ms")]
    pub startup_timeout_ms: u64,
    #[serde(default)]
    pub restart_policy: RestartPolicy,
    #[serde(default, skip_serializing_if = "RemoteSecurityConfig::is_empty")]
    pub remote_security: RemoteSecurityConfig,
}

impl std::fmt::Debug for ManagedServiceConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let env_keys = self.env.keys().collect::<std::collections::BTreeSet<_>>();
        formatter
            .debug_struct("ManagedServiceConfig")
            .field("name", &self.name)
            .field("mode", &self.mode)
            .field("binary", &self.binary)
            .field("args_count", &self.args.len())
            .field("url", &self.url.as_ref().map(|_| "[REDACTED_URL]"))
            .field("health_path", &self.health_path)
            .field("env_keys", &env_keys)
            .field("startup_timeout_ms", &self.startup_timeout_ms)
            .field("restart_policy", &self.restart_policy)
            .field("remote_security", &self.remote_security)
            .finish()
    }
}

pub(super) fn default_health_path() -> String {
    "/healthz".to_string()
}

pub(super) fn default_startup_timeout_ms() -> u64 {
    5_000
}

const fn default_max_restarts() -> u32 {
    3
}

const fn default_initial_backoff_ms() -> u64 {
    250
}

const fn default_max_backoff_ms() -> u64 {
    4_000
}
