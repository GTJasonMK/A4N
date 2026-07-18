use std::collections::HashMap;

use a4n_core::config::app_config::AppConfig;
use a4n_service_manager::{
    ManagedServiceConfig, ManagedSidecarProcess, ServiceManager, ServiceMode,
};

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
            .start_sidecar(name)
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
                if matches!(name, "prompt" | "import" | "export") {
                    ManagedServiceConfig::embedded(name)
                } else {
                    ManagedServiceConfig::disabled(name)
                }
            })
            .collect()),
        RuntimeProfile::Sidecar => {
            let managed_sidecars_enabled = managed_sidecars_enabled()?;
            sidecar_service_specs()
                .into_iter()
                .map(|spec| {
                    if managed_sidecars_enabled {
                        managed_sidecar_service(spec, config)
                    } else {
                        Ok(remote_service(spec.name, sidecar_url_from_spec(spec)?))
                    }
                })
                .collect()
        }
    }
}

fn managed_sidecar_service(
    spec: SidecarServiceSpec,
    app_config: &AppConfig,
) -> Result<ManagedServiceConfig, String> {
    let binary = sidecar_binary_path(spec.name).ok_or_else(|| {
        format!(
            "{MANAGED_SIDECARS_ENV}=1 需要可执行 sidecar 二进制，但未找到 {}；请设置 {SIDECAR_BIN_DIR_ENV} 或使用发布包 bin/services",
            sidecar_executable_name(spec.name).unwrap_or_else(|| spec.name.to_string())
        )
    })?;
    let mut config = ManagedServiceConfig::sidecar(spec.name, binary, sidecar_url_from_spec(spec)?);
    config.env.insert(
        spec.addr_env.to_string(),
        sidecar_addr(spec.addr_env, spec.default_addr)?,
    );
    apply_common_sidecar_env(&mut config.env, app_config)?;
    Ok(config)
}

fn remote_service(name: impl Into<String>, url: String) -> ManagedServiceConfig {
    let name = name.into();
    ManagedServiceConfig {
        binary: sidecar_binary_path(&name),
        name,
        mode: ServiceMode::Remote,
        args: Vec::new(),
        url: Some(url),
        health_path: "/healthz".to_string(),
        env: HashMap::new(),
        startup_timeout_ms: 5_000,
    }
}
