use super::config::{ManagedServiceConfig, ServiceMode};
use super::errors::ServiceManagerError;
use super::health::http_health;
use super::integrity::BinaryIntegrityVerifier;
use super::process::ManagedSidecarProcess;
use a4n_service_protocol::{SIDECAR_CONNECT_TIMEOUT, SIDECAR_HEALTH_TIMEOUT, ServiceHealth};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
use tokio::sync::{RwLock, watch};

pub(crate) type HttpClientFactory =
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

    pub fn auth_token(&self, name: &str) -> Result<Option<String>, ServiceManagerError> {
        configured_auth_token(self.config(name)?)
    }

    /// Build a capability client factory that reloads changed remote TLS material
    /// at request boundaries, while keeping local sidecars on one pool.
    pub fn capability_http_client_factory(
        &self,
        name: &str,
    ) -> Result<HttpClientFactory, ServiceManagerError> {
        let config = self.config(name)?.clone();
        match config.mode {
            ServiceMode::Remote => Ok(remote_client_factory(config)),
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
        let (client, token) = build_remote_client(config, true)?;
        http_health(&client, config, token.as_deref()).await
    }
}

fn remote_client_factory(config: ManagedServiceConfig) -> HttpClientFactory {
    let cache = Arc::new(Mutex::new(None));
    Arc::new(move || refresh_remote_client_cache(&cache, &config))
}

fn refresh_remote_client_cache(
    cache: &Mutex<Option<CachedRemoteClient>>,
    config: &ManagedServiceConfig,
) -> Result<reqwest::Client, String> {
    let fingerprint = remote_client_fingerprint(config)?;
    let mut cached = cache
        .lock()
        .map_err(|_| "remote HTTP client cache lock is poisoned".to_string())?;
    if let Some(entry) = cached.as_ref()
        && entry.fingerprint == fingerprint
    {
        return Ok(entry.client.clone());
    }

    let (client, _) = build_remote_client(config, false).map_err(|error| error.to_string())?;
    *cached = Some(CachedRemoteClient {
        fingerprint,
        client: client.clone(),
    });
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

    let token = configured_auth_token(config)?;

    let client = builder.build().map_err(|_| {
        ServiceManagerError::InvalidConfig(format!(
            "remote service {} HTTP client initialization failed",
            config.name
        ))
    })?;
    Ok((client, token))
}

pub(crate) fn configured_auth_token(
    config: &ManagedServiceConfig,
) -> Result<Option<String>, ServiceManagerError> {
    let env_name = config
        .remote_security
        .auth_token_env
        .as_deref()
        .or_else(|| {
            config
                .env
                .contains_key(a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV)
                .then_some(a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV)
        });
    let Some(env_name) = env_name else {
        return Ok(None);
    };
    let value = match config.env.get(env_name) {
        Some(value) => value.clone(),
        None => std::env::var(env_name).map_err(|_| {
            ServiceManagerError::InvalidConfig(format!(
                "service {} credential environment is unavailable",
                config.name
            ))
        })?,
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(ServiceManagerError::InvalidConfig(format!(
            "service {} credential environment is empty",
            config.name
        )));
    }
    Ok(Some(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::fingerprint_file;
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
}
