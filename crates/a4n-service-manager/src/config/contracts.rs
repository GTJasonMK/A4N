use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceMode {
    Embedded,
    Sidecar,
    Remote,
    Disabled,
}

#[derive(Debug, Clone, Copy)]
pub struct RestartPolicy {
    pub max_restarts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl RestartPolicy {
    pub(crate) fn backoff_ms(self, restart_count: u32) -> u64 {
        let shift = (restart_count - 1).min(31);
        self.initial_backoff_ms
            .saturating_mul(1_u64 << shift)
            .min(self.max_backoff_ms)
    }
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            initial_backoff_ms: 250,
            max_backoff_ms: 4_000,
        }
    }
}

/// 远程 sidecar 的独立部署凭证与 TLS 身份配置。
/// 只保存环境变量名和证书路径，不把 token/private key 内容放进配置合同。
#[derive(Debug, Clone, Default)]
pub struct RemoteSecurityConfig {
    pub auth_token_env: Option<String>,
    pub ca_cert_path: Option<PathBuf>,
    pub client_cert_path: Option<PathBuf>,
    pub client_key_path: Option<PathBuf>,
}

impl RemoteSecurityConfig {
    pub fn has_client_identity(&self) -> bool {
        self.client_cert_path.is_some() && self.client_key_path.is_some()
    }

    pub fn has_deployment_credential(&self) -> bool {
        self.auth_token_env.is_some() || self.has_client_identity()
    }
}

#[derive(Clone)]
pub struct ManagedServiceConfig {
    pub name: String,
    pub mode: ServiceMode,
    pub binary: Option<PathBuf>,
    pub url: Option<String>,
    pub env: HashMap<String, String>,
    pub startup_timeout_ms: u64,
    pub restart_policy: RestartPolicy,
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
            .field("url", &self.url.as_ref().map(|_| "[REDACTED_URL]"))
            .field("env_keys", &env_keys)
            .field("startup_timeout_ms", &self.startup_timeout_ms)
            .field("restart_policy", &self.restart_policy)
            .field("remote_security", &self.remote_security)
            .finish()
    }
}
