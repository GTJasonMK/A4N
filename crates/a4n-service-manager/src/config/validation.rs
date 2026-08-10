use super::contracts::{ManagedServiceConfig, RemoteSecurityConfig, ServiceMode};
use crate::errors::ServiceManagerError;
use a4n_service_protocol::{SIDECAR_AUTH_TOKEN_ENV, expected_capability_version_for_service};

impl ManagedServiceConfig {
    pub fn validate(&self) -> Result<(), ServiceManagerError> {
        if self.name.trim().is_empty() {
            return Err(ServiceManagerError::InvalidConfig(
                "service name cannot be blank".to_string(),
            ));
        }
        if !self.health_path.starts_with('/') {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "service {} health_path must start with /",
                self.name
            )));
        }
        if self.restart_policy.initial_backoff_ms == 0
            || self.restart_policy.max_backoff_ms < self.restart_policy.initial_backoff_ms
        {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "service {} restart backoff must be positive and max >= initial",
                self.name
            )));
        }

        match self.mode {
            ServiceMode::Embedded => self.validate_registered_local_mode("embedded"),
            ServiceMode::Disabled => self.validate_registered_local_mode("disabled"),
            ServiceMode::Remote => self.validate_remote(),
            ServiceMode::Sidecar => self.validate_sidecar(),
        }
    }

    fn validate_registered_local_mode(&self, mode: &str) -> Result<(), ServiceManagerError> {
        if expected_capability_version_for_service(&self.name).is_none() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "{mode} service {} has no registered capability version",
                self.name
            )));
        }
        Ok(())
    }

    fn validate_remote(&self) -> Result<(), ServiceManagerError> {
        let configured_url = self.url.as_deref().map(str::trim).unwrap_or("");
        if configured_url.is_empty() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {} requires url",
                self.name
            )));
        }
        let url = reqwest::Url::parse(configured_url).map_err(|_| {
            ServiceManagerError::InvalidConfig(format!(
                "remote service {} has invalid url",
                self.name
            ))
        })?;
        if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {} url must not contain credentials or fragment",
                self.name
            )));
        }
        let loopback = is_loopback_url(&url);
        if url.scheme() != "https" && !loopback {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {} must use HTTPS outside loopback",
                self.name
            )));
        }
        validate_remote_security(&self.remote_security, &self.name, !loopback)?;
        if !loopback && !self.remote_security.has_deployment_credential() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {} requires an independent auth_token_env or complete mTLS identity",
                self.name
            )));
        }
        Ok(())
    }

    fn validate_sidecar(&self) -> Result<(), ServiceManagerError> {
        if self.binary.is_none() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "sidecar service {} requires binary",
                self.name
            )));
        }
        if self.url.as_deref().map(str::trim).unwrap_or("").is_empty() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "sidecar service {} requires url",
                self.name
            )));
        }
        validate_sidecar_environment(&self.env, &self.name)?;
        Ok(())
    }
}

fn validate_sidecar_environment(
    environment: &std::collections::HashMap<String, String>,
    service: &str,
) -> Result<(), ServiceManagerError> {
    for key in environment.keys() {
        let normalized = key.to_ascii_uppercase();
        let host_secret = is_host_long_lived_secret_environment_key(&normalized)
            || (normalized.ends_with("_TOKEN") && key != SIDECAR_AUTH_TOKEN_ENV);
        if host_secret {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "service {service} environment key {key} is not allowed for sidecar launch"
            )));
        }
    }
    Ok(())
}

fn is_loopback_url(url: &reqwest::Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn validate_remote_security(
    security: &RemoteSecurityConfig,
    service: &str,
    requires_independent_credential: bool,
) -> Result<(), ServiceManagerError> {
    if let Some(name) = security.auth_token_env.as_deref() {
        let mut chars = name.chars();
        let valid = matches!(chars.next(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic())
            && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric());
        if name.trim().is_empty() || !valid {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {service} auth_token_env must be a valid environment variable name"
            )));
        }
        let normalized = name.to_ascii_uppercase();
        if requires_independent_credential
            && (normalized == SIDECAR_AUTH_TOKEN_ENV
                || is_host_long_lived_secret_environment_key(&normalized))
        {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {service} auth_token_env must name an independent remote deployment credential"
            )));
        }
    }
    if security.client_cert_path.is_some() != security.client_key_path.is_some() {
        return Err(ServiceManagerError::InvalidConfig(format!(
            "remote service {service} mTLS requires both client_cert_path and client_key_path"
        )));
    }
    Ok(())
}

fn is_host_long_lived_secret_environment_key(normalized: &str) -> bool {
    normalized.contains("DATABASE_URL")
        || normalized.contains("JWT_SECRET")
        || normalized.contains("BOOTSTRAP_ADMIN_PASSWORD")
        || normalized.ends_with("_API_KEY")
        || normalized.ends_with("_PASSWORD")
        || normalized.ends_with("_SECRET")
        || matches!(
            normalized,
            "A4N_TEMP_MEDIA_UPLOAD_TOKEN" | "A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN"
        )
}
