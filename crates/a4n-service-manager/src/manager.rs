use super::config::{ManagedServiceConfig, ServiceMode};
use super::errors::ServiceManagerError;
use super::health::http_health;
use super::integrity::BinaryIntegrityVerifier;
use super::process::ManagedSidecarProcess;
use a4n_service_protocol::{SIDECAR_CONNECT_TIMEOUT, SIDECAR_HEALTH_TIMEOUT, ServiceHealth};
use notify::{RecursiveMode, Watcher as NotifyWatcher};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::UNIX_EPOCH;
use tokio::sync::{RwLock, watch};

pub type HttpClientFactory =
    Arc<dyn Fn() -> Result<reqwest::Client, String> + Send + Sync + 'static>;

#[derive(Clone, PartialEq, Eq)]
struct RemoteFileFingerprint {
    modified: (u64, u32),
    length: u64,
    digest: [u8; 32],
}

#[derive(Clone, PartialEq, Eq)]
struct RemoteClientFingerprint {
    ca_cert: Option<RemoteFileFingerprint>,
    client_cert: Option<RemoteFileFingerprint>,
    client_key: Option<RemoteFileFingerprint>,
}

struct CachedRemoteClient {
    fingerprint: RemoteClientFingerprint,
    client: reqwest::Client,
}

struct RemoteClientFactoryState {
    cache: Mutex<Option<CachedRemoteClient>>,
    dirty: AtomicBool,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

#[derive(Debug)]
pub struct ServiceManager {
    configs: HashMap<String, ManagedServiceConfig>,
    client: reqwest::Client,
    capability_client: reqwest::Client,
    integrity: Option<BinaryIntegrityVerifier>,
    process_statuses:
        RwLock<HashMap<String, watch::Receiver<super::process::ManagedSidecarStatus>>>,
}

impl ServiceManager {
    pub fn new(configs: Vec<ManagedServiceConfig>) -> Result<Self, ServiceManagerError> {
        let mut by_name = HashMap::new();
        for config in configs {
            config.validate()?;
            if by_name.insert(config.name.clone(), config).is_some() {
                return Err(ServiceManagerError::InvalidConfig(
                    "duplicate service name".to_string(),
                ));
            }
        }
        let client = reqwest::Client::builder()
            .connect_timeout(SIDECAR_CONNECT_TIMEOUT)
            .timeout(SIDECAR_HEALTH_TIMEOUT)
            .build()
            .map_err(|err| {
                ServiceManagerError::InvalidConfig(format!(
                    "service health HTTP client init failed: {err}"
                ))
            })?;
        let capability_client = reqwest::Client::builder()
            .connect_timeout(SIDECAR_CONNECT_TIMEOUT)
            .build()
            .map_err(|err| {
                ServiceManagerError::InvalidConfig(format!(
                    "capability HTTP client init failed: {err}"
                ))
            })?;
        Ok(Self {
            configs: by_name,
            client,
            capability_client,
            integrity: BinaryIntegrityVerifier::from_env()?,
            process_statuses: RwLock::new(HashMap::new()),
        })
    }

    pub fn config(&self, name: &str) -> Result<&ManagedServiceConfig, ServiceManagerError> {
        self.configs
            .get(name)
            .ok_or_else(|| ServiceManagerError::UnknownService(name.to_string()))
    }

    pub fn configs(&self) -> Vec<ManagedServiceConfig> {
        let mut configs = self.configs.values().cloned().collect::<Vec<_>>();
        configs.sort_by(|left, right| left.name.cmp(&right.name));
        configs
    }

    pub fn auth_token(&self, name: &str) -> Option<String> {
        let config = self.configs.get(name)?;
        let env_name = config
            .remote_security
            .auth_token_env
            .as_deref()
            .or_else(|| {
                config
                    .env
                    .contains_key(a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV)
                    .then_some(a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV)
            })?;
        config
            .env
            .get(env_name)
            .cloned()
            .or_else(|| std::env::var(env_name).ok())
            .filter(|token| !token.trim().is_empty())
    }

    pub fn http_client(&self, name: &str) -> Result<reqwest::Client, ServiceManagerError> {
        let config = self.config(name)?;
        match config.mode {
            ServiceMode::Remote => self.remote_client(config, true).map(|(client, _)| client),
            ServiceMode::Sidecar => Ok(self.client.clone()),
            _ => Err(ServiceManagerError::InvalidConfig(format!(
                "service {} does not use HTTP transport",
                config.name
            ))),
        }
    }

    /// Build the client used by capability operations.
    ///
    /// Health checks intentionally use a short total timeout, while capability
    /// transport applies the shared operation/stream deadlines at its own
    /// boundary. Reusing the health client here would silently cap long-running
    /// generation and import/export requests at the health timeout.
    pub fn capability_http_client(
        &self,
        name: &str,
    ) -> Result<reqwest::Client, ServiceManagerError> {
        let config = self.config(name)?;
        match config.mode {
            ServiceMode::Remote => self.remote_client(config, false).map(|(client, _)| client),
            ServiceMode::Sidecar => Ok(self.capability_client.clone()),
            _ => Err(ServiceManagerError::InvalidConfig(format!(
                "service {} does not use HTTP transport",
                config.name
            ))),
        }
    }

    /// Build a capability client factory that reloads remote TLS material when
    /// its certificate files change, while keeping local sidecars on one pool.
    pub fn capability_http_client_factory(
        &self,
        name: &str,
    ) -> Result<HttpClientFactory, ServiceManagerError> {
        let config = self.config(name)?.clone();
        match config.mode {
            ServiceMode::Remote => remote_client_factory(config, false),
            ServiceMode::Sidecar => {
                let client = self.capability_client.clone();
                Ok(Arc::new(move || Ok(client.clone())))
            }
            _ => Err(ServiceManagerError::InvalidConfig(format!(
                "service {} does not use HTTP transport",
                config.name
            ))),
        }
    }

    pub async fn health(&self, name: &str) -> Result<ServiceHealth, ServiceManagerError> {
        let config = self.config(name)?;
        match config.mode {
            ServiceMode::Embedded => config.embedded_health(),
            ServiceMode::Disabled => {
                Err(ServiceManagerError::CapabilityDisabled(config.name.clone()))
            }
            ServiceMode::Remote => self.http_health(config).await,
            ServiceMode::Sidecar => http_health(&self.client, config, None).await,
        }
    }

    pub async fn process_status(
        &self,
        name: &str,
    ) -> Result<Option<super::process::ManagedSidecarStatus>, ServiceManagerError> {
        self.config(name)?;
        Ok(self
            .process_statuses
            .read()
            .await
            .get(name)
            .map(|status| status.borrow().clone()))
    }

    pub async fn start_sidecar(
        &self,
        name: &str,
    ) -> Result<ManagedSidecarProcess, ServiceManagerError> {
        let config = self.config(name)?.clone();
        if config.mode != ServiceMode::Sidecar {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "service {} is not configured as sidecar",
                config.name
            )));
        }
        config.validate()?;
        if let Some(status) = self.process_status(&config.name).await?
            && !matches!(
                status.state,
                super::process::ManagedSidecarState::Stopped
                    | super::process::ManagedSidecarState::CircuitOpen
            )
        {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "service {} is already managed",
                config.name
            )));
        }
        let process =
            ManagedSidecarProcess::start(config, self.client.clone(), self.integrity.clone())
                .await?;
        self.process_statuses.write().await.insert(
            process.service_name().to_string(),
            process.status_receiver(),
        );
        Ok(process)
    }

    async fn http_health(
        &self,
        config: &ManagedServiceConfig,
    ) -> Result<ServiceHealth, ServiceManagerError> {
        let (client, token) = self.remote_client(config, true)?;
        http_health(&client, config, token.as_deref()).await
    }

    fn remote_client(
        &self,
        config: &ManagedServiceConfig,
        health_client: bool,
    ) -> Result<(reqwest::Client, Option<String>), ServiceManagerError> {
        build_remote_client(config, health_client)
    }
}

fn remote_client_factory(
    config: ManagedServiceConfig,
    health_client: bool,
) -> Result<HttpClientFactory, ServiceManagerError> {
    let state = Arc::new(RemoteClientFactoryState {
        cache: Mutex::new(None),
        dirty: AtomicBool::new(false),
        watcher: Mutex::new(None),
    });
    let watcher = start_remote_tls_watcher(&config, health_client, Arc::downgrade(&state));
    if watcher.is_none() && has_remote_tls_material(&config) {
        tracing::warn!(
            service = %config.name,
            "remote TLS file watcher unavailable; request-boundary fingerprint polling remains active"
        );
    }
    *state.watcher.lock().map_err(|_| {
        ServiceManagerError::InvalidConfig("remote TLS watcher lock poisoned".into())
    })? = watcher;
    let state_for_factory = Arc::clone(&state);
    Ok(Arc::new(move || {
        refresh_remote_client_cache(&state_for_factory, &config, health_client)
    }))
}

fn has_remote_tls_material(config: &ManagedServiceConfig) -> bool {
    let security = &config.remote_security;
    security.ca_cert_path.is_some()
        || security.client_cert_path.is_some()
        || security.client_key_path.is_some()
}

fn start_remote_tls_watcher(
    config: &ManagedServiceConfig,
    health_client: bool,
    state: Weak<RemoteClientFactoryState>,
) -> Option<notify::RecommendedWatcher> {
    let directories = [
        config.remote_security.ca_cert_path.as_deref(),
        config.remote_security.client_cert_path.as_deref(),
        config.remote_security.client_key_path.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(|path| {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    })
    .collect::<std::collections::BTreeSet<_>>();
    if directories.is_empty() {
        return None;
    }

    let callback_config = config.clone();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Some(state) = state.upgrade() else {
            return;
        };
        state.dirty.store(true, Ordering::Release);
        if result.is_ok() {
            let _ = refresh_remote_client_cache(&state, &callback_config, health_client);
        }
    })
    .ok()?;
    for directory in directories {
        if watcher
            .watch(&directory, RecursiveMode::NonRecursive)
            .is_err()
        {
            return None;
        }
    }
    Some(watcher)
}

fn refresh_remote_client_cache(
    state: &RemoteClientFactoryState,
    config: &ManagedServiceConfig,
    health_client: bool,
) -> Result<reqwest::Client, String> {
    let fingerprint = remote_client_fingerprint(config)?;
    let mut cached = state
        .cache
        .lock()
        .map_err(|_| "remote HTTP client cache lock is poisoned".to_string())?;
    if let Some(entry) = cached.as_ref()
        && entry.fingerprint == fingerprint
    {
        state.dirty.store(false, Ordering::Release);
        return Ok(entry.client.clone());
    }

    let (client, _) =
        build_remote_client(config, health_client).map_err(|error| error.to_string())?;
    *cached = Some(CachedRemoteClient {
        fingerprint,
        client: client.clone(),
    });
    state.dirty.store(false, Ordering::Release);
    Ok(client)
}

fn remote_client_fingerprint(
    config: &ManagedServiceConfig,
) -> Result<RemoteClientFingerprint, String> {
    Ok(RemoteClientFingerprint {
        ca_cert: fingerprint_file(config.remote_security.ca_cert_path.as_deref())?,
        client_cert: fingerprint_file(config.remote_security.client_cert_path.as_deref())?,
        client_key: fingerprint_file(config.remote_security.client_key_path.as_deref())?,
    })
}

fn fingerprint_file(path: Option<&Path>) -> Result<Option<RemoteFileFingerprint>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let bytes = std::fs::read(path)
        .map_err(|_| "remote TLS certificate material is unavailable".to_string())?;
    let metadata = std::fs::metadata(path)
        .map_err(|_| "remote TLS certificate material is unavailable".to_string())?;
    let modified = metadata
        .modified()
        .map_err(|_| "remote TLS certificate material timestamp is unavailable".to_string())?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "remote TLS certificate material timestamp is invalid".to_string())?;
    Ok(Some(RemoteFileFingerprint {
        modified: (modified.as_secs(), modified.subsec_nanos()),
        length: metadata.len(),
        digest: Sha256::digest(bytes).into(),
    }))
}

fn build_remote_client(
    config: &ManagedServiceConfig,
    health_client: bool,
) -> Result<(reqwest::Client, Option<String>), ServiceManagerError> {
    let security = &config.remote_security;
    let mut builder = reqwest::Client::builder().connect_timeout(SIDECAR_CONNECT_TIMEOUT);
    if health_client {
        builder = builder.timeout(SIDECAR_HEALTH_TIMEOUT);
    }

    if let Some(path) = security.ca_cert_path.as_ref() {
        let pem = std::fs::read(path).map_err(|_| {
            ServiceManagerError::InvalidConfig(format!(
                "remote service {} CA certificate is unavailable",
                config.name
            ))
        })?;
        let certificate = reqwest::Certificate::from_pem(&pem).map_err(|_| {
            ServiceManagerError::InvalidConfig(format!(
                "remote service {} CA certificate is invalid",
                config.name
            ))
        })?;
        builder = builder.add_root_certificate(certificate);
    }

    if security.has_client_identity() {
        let cert = std::fs::read(
            security
                .client_cert_path
                .as_ref()
                .expect("validated client certificate path"),
        )
        .map_err(|_| {
            ServiceManagerError::InvalidConfig(format!(
                "remote service {} client certificate is unavailable",
                config.name
            ))
        })?;
        let key = std::fs::read(
            security
                .client_key_path
                .as_ref()
                .expect("validated client key path"),
        )
        .map_err(|_| {
            ServiceManagerError::InvalidConfig(format!(
                "remote service {} client key is unavailable",
                config.name
            ))
        })?;
        let identity = reqwest::Identity::from_pkcs8_pem(&cert, &key).map_err(|_| {
            ServiceManagerError::InvalidConfig(format!(
                "remote service {} client TLS identity is invalid",
                config.name
            ))
        })?;
        builder = builder.identity(identity);
    }

    let token = security
        .auth_token_env
        .as_deref()
        .map(|env_name| {
            config
                .env
                .get(env_name)
                .cloned()
                .or_else(|| std::env::var(env_name).ok())
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    ServiceManagerError::InvalidConfig(format!(
                        "remote service {} credential environment is unavailable",
                        config.name
                    ))
                })
        })
        .transpose()?;

    let client = builder.build().map_err(|_| {
        ServiceManagerError::InvalidConfig(format!(
            "remote service {} HTTP client initialization failed",
            config.name
        ))
    })?;
    Ok((client, token))
}

#[cfg(test)]
mod tests {
    use super::{RemoteClientFactoryState, fingerprint_file, start_remote_tls_watcher};
    use crate::ManagedServiceConfig;
    use std::sync::atomic::Ordering;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn remote_tls_fingerprint_detects_same_length_content_rotation() {
        let path = std::env::temp_dir().join(format!(
            "a4n-remote-tls-fingerprint-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::write(&path, b"aaaa").expect("fingerprint fixture should be written");
        let before = fingerprint_file(Some(&path))
            .expect("initial TLS fingerprint should be readable")
            .expect("configured TLS file should have a fingerprint");

        std::fs::write(&path, b"bbbb").expect("rotated TLS fixture should be written");
        let after = fingerprint_file(Some(&path))
            .expect("rotated TLS fingerprint should be readable")
            .expect("configured TLS file should retain a fingerprint");

        assert_eq!(before.length, after.length);
        assert_ne!(before.digest, after.digest);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn remote_tls_watcher_marks_certificate_directory_changes() {
        let dir = std::env::temp_dir().join(format!(
            "a4n-remote-tls-watcher-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("watcher fixture directory should be created");
        let cert = dir.join("client.crt");
        std::fs::write(&cert, b"initial").expect("watcher fixture should be written");
        let mut config = ManagedServiceConfig::remote("prompt", "https://prompt.example.test");
        config.remote_security.client_cert_path = Some(cert.clone());
        config.remote_security.client_key_path = Some(dir.join("client.key"));
        std::fs::write(
            config
                .remote_security
                .client_key_path
                .as_ref()
                .expect("watcher key path should be configured"),
            b"key",
        )
        .expect("watcher key fixture should be written");

        let state = std::sync::Arc::new(RemoteClientFactoryState {
            cache: std::sync::Mutex::new(None),
            dirty: std::sync::atomic::AtomicBool::new(false),
            watcher: std::sync::Mutex::new(None),
        });
        let watcher = start_remote_tls_watcher(&config, false, std::sync::Arc::downgrade(&state))
            .expect("platform watcher should start for a real temporary directory");
        std::fs::write(&cert, b"changed").expect("watcher fixture should rotate");
        let observed = (0..80).any(|_| {
            if state.dirty.load(Ordering::Acquire) {
                true
            } else {
                std::thread::sleep(std::time::Duration::from_millis(25));
                false
            }
        });
        drop(watcher);
        let _ = std::fs::remove_dir_all(dir);
        assert!(
            observed,
            "platform watcher should observe certificate changes"
        );
    }
}
