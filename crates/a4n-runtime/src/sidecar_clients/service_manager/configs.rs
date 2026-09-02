use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener};

use a4n_core::config::app_config::AppConfig;
use a4n_core::config::env::{optional_non_empty_env_path, optional_non_empty_env_string};
use a4n_service_manager::{
    ManagedServiceConfig, ManagedSidecarProcess, RemoteSecurityConfig, ServiceManager,
    is_loopback_http_url,
};
use a4n_service_protocol::{
    EXPORT_SERVICE_NAME, IMPORT_SERVICE_NAME, PROMPT_SERVICE_NAME, SIDECAR_AUTH_TOKEN_ENV,
    ServiceCatalogEntry,
};

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

pub(crate) async fn start_managed_sidecars(
    profile: RuntimeProfile,
    service_manager: &ServiceManager,
) -> Result<Vec<ManagedSidecarProcess>, String> {
    if !profile.uses_sidecars() || !managed_sidecars_enabled()? {
        return Ok(Vec::new());
    }

    let results = futures::future::join_all(capability_service_names().into_iter().map(
        |name| async move {
            let result = service_manager
                .start_sidecar(&name)
                .await
                .map_err(|err| err.to_string());
            (name, result)
        },
    ))
    .await;

    let mut processes = Vec::with_capacity(results.len());
    let mut first_error = None;
    for (_, result) in results {
        match result {
            Ok(process) => processes.push(process),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }

    if let Some(error) = first_error {
        let cleanup_failures = futures::future::join_all(
            processes
                .into_iter()
                .map(a4n_service_manager::ManagedSidecarProcess::force_stop),
        )
        .await
        .into_iter()
        .filter_map(Result::err)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
        if !cleanup_failures.is_empty() {
            tracing::error!(
                primary_error = %error,
                cleanup_errors = %cleanup_failures.join("; "),
                "managed sidecar startup failed and cleanup was incomplete"
            );
        }
        return Err(error);
    }

    Ok(processes)
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
            let mut reserved_addrs = HashSet::new();
            sidecar_service_specs()
                .into_iter()
                .map(|spec| {
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
