use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, TcpListener};

use a4n_core::config::app_config::AppConfig;
use a4n_core::config::env::{optional_non_empty_env_path, optional_non_empty_env_string};
use a4n_service_manager::{
    ManagedServiceConfig, ManagedSidecarProcess, RemoteSecurityConfig, ServiceManager, ServiceMode,
};
use a4n_service_protocol::SIDECAR_AUTH_TOKEN_ENV;

use crate::state::RuntimeProfile;

use super::binaries::{sidecar_binary_path, sidecar_executable_name};
use super::environment::apply_common_sidecar_env;
use super::specs::{
    MANAGED_SIDECARS_ENV, SIDECAR_BIN_DIR_ENV, SidecarServiceSpec, capability_service_names,
    managed_sidecars_enabled, sidecar_addr, sidecar_service_specs, sidecar_url_from_spec,
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

    let mut processes = Vec::new();
    for name in capability_service_names() {
        let process = service_manager
            .start_sidecar(&name)
            .await
            .map_err(|err| err.to_string())?;
        processes.push(process);
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
                if matches!(name.as_str(), "prompt" | "import" | "export") {
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
                        remote_service(spec.name.clone(), sidecar_url_from_spec(spec)?, config)
                    }
                })
                .collect()
        }
    }
}

fn managed_sidecar_service(
    spec: SidecarServiceSpec,
    app_config: &AppConfig,
    reserved_addrs: &mut HashSet<SocketAddr>,
) -> Result<ManagedServiceConfig, String> {
    let binary = sidecar_binary_path(&spec.name).ok_or_else(|| {
        format!(
            "{MANAGED_SIDECARS_ENV}=1 需要可执行 sidecar 二进制，但未找到 {}；请设置 {SIDECAR_BIN_DIR_ENV} 或使用发布包 bin/services",
            sidecar_executable_name(&spec.name).unwrap_or_else(|| spec.name.clone())
        )
    })?;
    let addr = match std::env::var(&spec.addr_env) {
        Ok(_) => sidecar_addr(&spec.addr_env, &spec.default_addr)?,
        Err(std::env::VarError::NotPresent) => {
            reserve_managed_loopback_addr(reserved_addrs)?.to_string()
        }
        Err(error) => return Err(format!("{} 无法读取: {error}", spec.addr_env)),
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

fn remote_service(
    name: impl Into<String>,
    url: String,
    app_config: &AppConfig,
) -> Result<ManagedServiceConfig, String> {
    let name = name.into();
    let remote_security = remote_security_from_env(&name)?;
    let local_auth_token = remote_security
        .is_empty()
        .then(|| external_sidecar_auth_token(app_config))
        .transpose()?;
    Ok(build_remote_service(
        name,
        url,
        remote_security,
        local_auth_token.as_deref(),
    ))
}

pub(super) fn build_remote_service(
    name: impl Into<String>,
    url: String,
    remote_security: RemoteSecurityConfig,
    local_auth_token: Option<&str>,
) -> ManagedServiceConfig {
    let name = name.into();
    let mut config = ManagedServiceConfig {
        binary: sidecar_binary_path(&name),
        name,
        mode: ServiceMode::Remote,
        args: Vec::new(),
        url: Some(url),
        health_path: "/healthz".to_string(),
        env: HashMap::new(),
        startup_timeout_ms: 5_000,
        restart_policy: Default::default(),
        remote_security,
    };
    if config.remote_security.is_empty() {
        config.remote_security.auth_token_env = Some(SIDECAR_AUTH_TOKEN_ENV.to_string());
        if let Some(token) = local_auth_token {
            config
                .env
                .insert(SIDECAR_AUTH_TOKEN_ENV.to_string(), token.to_string());
        }
    }
    config
}

fn remote_security_from_env(service: &str) -> Result<RemoteSecurityConfig, String> {
    let prefix = remote_security_env_prefix(service);
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

pub(super) fn remote_security_env_prefix(service: &str) -> String {
    format!("A4N_REMOTE_{}", service.to_ascii_uppercase())
}

fn external_sidecar_auth_token(config: &AppConfig) -> Result<String, String> {
    if let Ok(token) = std::env::var(SIDECAR_AUTH_TOKEN_ENV) {
        let token = token.trim();
        if !token.is_empty() {
            return Ok(token.to_string());
        }
    }
    let token_path = config.data_dir.join("services").join("auth-token");
    let token = std::fs::read_to_string(&token_path).map_err(|error| {
        format!(
            "script-owned sidecar 缺少认证 token；请设置 {SIDECAR_AUTH_TOKEN_ENV} 或由启动脚本创建 {}: {error}",
            token_path.display()
        )
    })?;
    let token = token.trim();
    if token.is_empty() {
        return Err(format!(
            "sidecar 认证 token 文件为空: {}",
            token_path.display()
        ));
    }
    Ok(token.to_string())
}
