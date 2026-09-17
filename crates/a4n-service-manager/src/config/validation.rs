use super::contracts::{ManagedServiceConfig, RemoteSecurityConfig, ServiceMode};
use crate::errors::ServiceManagerError;
use a4n_process_catalog::service_catalog_entry;
use a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV;

impl ManagedServiceConfig {
    pub fn validate(&self) -> Result<(), ServiceManagerError> {
        if self.name.trim().is_empty() {
            return Err(ServiceManagerError::InvalidConfig(
                "service name cannot be blank".to_string(),
            ));
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
        if service_catalog_entry(&self.name).is_none() {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "{mode} service {} has no registered capability version",
                self.name
            )));
        }
        Ok(())
    }

    fn validate_remote(&self) -> Result<(), ServiceManagerError> {
        if service_catalog_entry(&self.name).is_some_and(|service| service.local_only) {
            return Err(ServiceManagerError::InvalidConfig(format!(
                "remote service {} requires access to the Host local filesystem and must run as a managed sidecar",
                self.name
            )));
        }
        let url =
            parse_remote_service_url(self.url.as_deref().unwrap_or("")).map_err(|reason| {
                ServiceManagerError::InvalidConfig(format!("remote service {} {reason}", self.name))
            })?;
        let loopback = is_loopback_url(&url);
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
        parse_remote_service_url(self.url.as_deref().unwrap_or("")).map_err(|reason| {
            ServiceManagerError::InvalidConfig(format!("sidecar service {} {reason}", self.name))
        })?;
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

pub fn normalize_remote_service_url(value: &str) -> Result<String, String> {
    parse_remote_service_url(value).map(|url| url.as_str().trim_end_matches('/').to_string())
}

pub fn is_loopback_http_url(value: &str) -> bool {
    parse_remote_service_url(value).is_ok_and(|url| is_loopback_url(&url))
}

fn parse_remote_service_url(value: &str) -> Result<reqwest::Url, String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err("requires url".to_string());
    }
    let url = reqwest::Url::parse(value).map_err(|_| "has invalid url".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("url must use http or https".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("url must not contain credentials".to_string());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("url must not contain query or fragment".to_string());
    }
    if url.scheme() != "https" && !is_loopback_url(&url) {
        return Err("must use HTTPS outside loopback".to_string());
    }
    Ok(url)
}

fn is_loopback_url(url: &reqwest::Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.trim_end_matches('.');
    host.eq_ignore_ascii_case("localhost")
        || host.to_ascii_lowercase().ends_with(".localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
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
