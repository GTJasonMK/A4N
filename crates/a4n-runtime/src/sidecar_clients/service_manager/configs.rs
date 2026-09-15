use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;

use a4n_config::app_config::AppConfig;
use a4n_config::{optional_non_empty_env_path, optional_non_empty_env_string};
use a4n_export_contract::EXPORT_SERVICE_NAME;
use a4n_import_contract::IMPORT_SERVICE_NAME;
use a4n_process_catalog::{ReleaseManifest, ServiceCatalogEntry};
use a4n_service_manager::{
    ManagedServiceConfig, RemoteSecurityConfig, ServiceManager, is_loopback_http_url,
};
use a4n_service_protocol::{PROMPT_SERVICE_NAME, RELEASE_MANIFEST_ENV, SIDECAR_AUTH_TOKEN_ENV};

use crate::state::RuntimeProfile;

use super::binaries::{sidecar_binary_path, sidecar_executable_name};
use super::environment::apply_common_sidecar_env;
use super::specs::{
    MANAGED_SIDECARS_ENV, SIDECAR_BIN_DIR_ENV, capability_service_names, managed_sidecars_enabled,
    parse_sidecar_addr_env, sidecar_service_specs, sidecar_url_from_spec,
};

pub(crate) fn runtime_service_manager(
    profile: RuntimeProfile,
    config: &AppConfig,
) -> Result<ServiceManager, String> {
    ServiceManager::new(runtime_service_configs(profile, config)?).map_err(|err| err.to_string())
}

pub(super) fn runtime_service_configs(
    profile: RuntimeProfile,
    config: &AppConfig,
) -> Result<Vec<ManagedServiceConfig>, String> {
    match profile {
        RuntimeProfile::Full => Ok(capability_service_names()
            .into_iter()
            .map(ManagedServiceConfig::embedded)
            .collect()),
        RuntimeProfile::Minimal => Ok(capability_service_names()
            .into_iter()
            .map(|name| {
                if matches!(
                    name.as_str(),
                    PROMPT_SERVICE_NAME | IMPORT_SERVICE_NAME | EXPORT_SERVICE_NAME
                ) {
                    ManagedServiceConfig::embedded(name)
                } else {
                    ManagedServiceConfig::disabled(name)
                }
            })
            .collect()),
        RuntimeProfile::Sidecar => {
            let managed_sidecars_enabled = managed_sidecars_enabled()?;
            let selected_services = selected_service_names()?;
            let mut reserved_addrs = HashSet::new();
            sidecar_service_specs()
                .into_iter()
                .map(|spec| {
                    if !selected_services.contains(&spec.name) {
                        return Ok(ManagedServiceConfig::disabled(spec.name));
                    }
                    if managed_sidecars_enabled {
                        managed_sidecar_service(spec, config, &mut reserved_addrs)
                    } else {
                        remote_service(spec)
                    }
                })
                .collect()
        }
    }
}

fn selected_service_names() -> Result<HashSet<String>, String> {
    let all = capability_service_names()
        .into_iter()
        .collect::<HashSet<_>>();
    let Some(path) = std::env::var_os(RELEASE_MANIFEST_ENV).map(PathBuf::from) else {
        return Ok(all);
    };
    let bytes =
        std::fs::read(&path).map_err(|error| format!("读取 release manifest 失败: {error}"))?;
    let manifest: ReleaseManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("解析 release manifest 失败: {error}"))?;
    if manifest.services.is_empty() {
        return Ok(all);
    }
    let services = manifest.services;
    let selected = services.iter().cloned().collect::<HashSet<_>>();
    if selected.len() != services.len() || selected.iter().any(|name| !all.contains(name)) {
        return Err("release manifest services 包含未知或重复 service".to_string());
    }
    Ok(selected)
}

fn managed_sidecar_service(
    spec: ServiceCatalogEntry,
    app_config: &AppConfig,
    reserved_addrs: &mut HashSet<SocketAddr>,
) -> Result<ManagedServiceConfig, String> {
    let executable = sidecar_executable_name(&spec.binary);
    let binary = sidecar_binary_path(&spec.binary)?.ok_or_else(|| {
        format!(
            "{MANAGED_SIDECARS_ENV}=1 需要可执行 sidecar 二进制，但未找到 {}；请设置 {SIDECAR_BIN_DIR_ENV} 或使用发布包 bin/services",
            executable
        )
    })?;
    let configured_addr =
        optional_non_empty_env_string(&spec.addr_env).map_err(|error| error.to_string())?;
    let addr = match configured_addr.as_deref() {
        Some(value) => parse_sidecar_addr_env(&spec.addr_env, Some(value), &spec.default_addr)?,
        None => reserve_managed_loopback_addr(reserved_addrs)?.to_string(),
    };
    let mut config = ManagedServiceConfig::sidecar(&spec.name, binary, format!("http://{addr}"));
    config.env.insert(spec.addr_env.clone(), addr);
    config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        uuid::Uuid::new_v4().simple().to_string(),
    );
    apply_common_sidecar_env(&mut config.env, app_config)?;
    Ok(config)
}

pub(super) fn reserve_managed_loopback_addr(
    reserved_addrs: &mut HashSet<SocketAddr>,
) -> Result<SocketAddr, String> {
    // ponytail: pre-bind then release keeps the existing Command-based launcher; use held
    // listeners/FD passing only if hostile local port races become a real deployment concern.
    loop {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("无法为 managed sidecar 分配本地端口: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("无法读取 managed sidecar 本地端口: {error}"))?;
        if reserved_addrs.insert(addr) {
            return Ok(addr);
        }
    }
}

fn remote_service(spec: ServiceCatalogEntry) -> Result<ManagedServiceConfig, String> {
    let url = sidecar_url_from_spec(&spec)?;
    let mut remote_security = remote_security_from_env(&spec.name)?;
    if !remote_security.has_deployment_credential() && is_loopback_http_url(&url) {
        optional_non_empty_env_string(SIDECAR_AUTH_TOKEN_ENV)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| {
                format!("外部 Sidecar 缺少认证 token；请设置 {SIDECAR_AUTH_TOKEN_ENV}")
            })?;
        remote_security.auth_token_env = Some(SIDECAR_AUTH_TOKEN_ENV.to_string());
    }
    let mut config = ManagedServiceConfig::remote(spec.name, url);
    config.remote_security = remote_security;
    Ok(config)
}

fn remote_security_from_env(service: &str) -> Result<RemoteSecurityConfig, String> {
    let prefix = format!("A4N_REMOTE_{}", service.to_ascii_uppercase());
    let auth_token_env = format!("{prefix}_TOKEN");
    let auth_token_env = optional_non_empty_env_string(&auth_token_env)
        .map_err(|error| error.to_string())?
        .map(|_| auth_token_env);
    let ca_cert_path = remote_security_path(&prefix, "CA_CERT_PATH")?;
    let client_cert_path = remote_security_path(&prefix, "CLIENT_CERT_PATH")?;
    let client_key_path = remote_security_path(&prefix, "CLIENT_KEY_PATH")?;
    Ok(RemoteSecurityConfig {
        auth_token_env,
        ca_cert_path,
        client_cert_path,
        client_key_path,
    })
}

fn remote_security_path(prefix: &str, suffix: &str) -> Result<Option<std::path::PathBuf>, String> {
    optional_non_empty_env_path(&format!("{prefix}_{suffix}")).map_err(|error| error.to_string())
}
