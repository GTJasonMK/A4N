use std::collections::HashMap;
use std::path::PathBuf;

use super::contracts::{ManagedServiceConfig, ServiceMode};

impl ManagedServiceConfig {
    pub fn embedded(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            mode: ServiceMode::Embedded,
            binary: None,
            url: None,
            env: HashMap::new(),
            startup_timeout_ms: 5_000,
            restart_policy: Default::default(),
            remote_security: Default::default(),
        }
    }

    pub fn disabled(name: impl Into<String>) -> Self {
        Self {
            mode: ServiceMode::Disabled,
            ..Self::embedded(name)
        }
    }

    pub fn remote(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            mode: ServiceMode::Remote,
            url: Some(url.into()),
            ..Self::embedded(name)
        }
    }

    pub fn sidecar(
        name: impl Into<String>,
        binary: impl Into<PathBuf>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            mode: ServiceMode::Sidecar,
            binary: Some(binary.into()),
            url: Some(url.into()),
            ..Self::embedded(name)
        }
    }
}
