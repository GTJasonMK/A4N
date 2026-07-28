use std::collections::HashMap;
use std::path::PathBuf;

use super::contracts::{
    ManagedServiceConfig, ServiceMode, default_health_path, default_startup_timeout_ms,
};

impl ManagedServiceConfig {
    pub fn embedded(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            mode: ServiceMode::Embedded,
            binary: None,
            args: Vec::new(),
            url: None,
            health_path: default_health_path(),
            env: HashMap::new(),
            startup_timeout_ms: default_startup_timeout_ms(),
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
            name: name.into(),
            mode: ServiceMode::Remote,
            binary: None,
            args: Vec::new(),
            url: Some(url.into()),
            health_path: default_health_path(),
            env: HashMap::new(),
            startup_timeout_ms: default_startup_timeout_ms(),
            restart_policy: Default::default(),
            remote_security: Default::default(),
        }
    }

    pub fn sidecar(
        name: impl Into<String>,
        binary: impl Into<PathBuf>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            mode: ServiceMode::Sidecar,
            binary: Some(binary.into()),
            args: Vec::new(),
            url: Some(url.into()),
            health_path: default_health_path(),
            env: HashMap::new(),
            startup_timeout_ms: default_startup_timeout_ms(),
            restart_policy: Default::default(),
            remote_security: Default::default(),
        }
    }
}
