use a4n_core::config::env::{env_flag_enabled, optional_non_empty_env_string};
use a4n_service_manager::normalize_remote_service_url;

use super::catalog::{MANAGED_SIDECARS_ENV, SidecarServiceSpec, sidecar_service_specs};

pub(in crate::sidecar_clients) fn sidecar_service_and_url_for_capability(
    capability_version: &str,
) -> Result<(&'static str, String), String> {
    let service = a4n_service_protocol::service_for_capability_version(capability_version)
        .ok_or_else(|| format!("未注册的 sidecar capability_version: {capability_version}"))?;
    Ok((service, sidecar_url_for_service(service)?))
}

fn sidecar_url_for_service(name: &str) -> Result<String, String> {
    let spec = sidecar_service_specs()
        .into_iter()
        .find(|spec| spec.name == name)
        .ok_or_else(|| format!("未知 sidecar service: {name}"))?;
    sidecar_url_from_spec(&spec)
}

pub(in crate::sidecar_clients::service_manager) fn sidecar_url_from_spec(
    spec: &SidecarServiceSpec,
) -> Result<String, String> {
    if let Some(url) =
        optional_non_empty_env_string(&spec.url_env).map_err(|error| error.to_string())?
    {
        return normalize_remote_service_url(&url)
            .map_err(|error| format!("{} {error}", spec.url_env));
    }
    let addr = parse_sidecar_addr_env(
        &spec.addr_env,
        optional_non_empty_env_string(&spec.addr_env)
            .map_err(|error| error.to_string())?
            .as_deref(),
        &spec.default_addr,
    )?;
    Ok(format!("http://{addr}"))
}

pub(in crate::sidecar_clients::service_manager) fn managed_sidecars_enabled() -> Result<bool, String>
{
    env_flag_enabled(MANAGED_SIDECARS_ENV).map_err(|error| error.to_string())
}

pub(in crate::sidecar_clients::service_manager) fn parse_sidecar_addr_env(
    env_key: &str,
    value: Option<&str>,
    default_addr: &str,
) -> Result<String, String> {
    let value = value.unwrap_or(default_addr);
    a4n_service_protocol::parse_service_socket_addr(env_key, value).map(|addr| addr.to_string())
}
