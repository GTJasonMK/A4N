use super::catalog::{MANAGED_SIDECARS_ENV, SidecarServiceSpec, sidecar_service_specs};

pub(in crate::sidecar_clients) fn sidecar_url_for_service(name: &str) -> Result<String, String> {
    let spec = sidecar_service_specs()
        .into_iter()
        .find(|spec| spec.name == name)
        .ok_or_else(|| format!("未知 sidecar service: {name}"))?;
    sidecar_url_from_spec(spec)
}

pub(in crate::sidecar_clients::service_manager) fn sidecar_url_from_spec(
    spec: SidecarServiceSpec,
) -> Result<String, String> {
    if let Some(url) =
        parse_sidecar_url_env(spec.url_env.as_str(), optional_env_string(&spec.url_env)?)?
    {
        return Ok(url);
    }
    let addr = parse_sidecar_addr_env(
        &spec.addr_env,
        optional_env_string(&spec.addr_env)?.as_deref(),
        &spec.default_addr,
    )?;
    if addr == spec.default_addr {
        Ok(spec.default_url)
    } else {
        Ok(format!("http://{addr}"))
    }
}

pub(in crate::sidecar_clients::service_manager) fn sidecar_addr(
    env_key: &str,
    default_addr: &str,
) -> Result<String, String> {
    let value = optional_env_string(env_key)?;
    parse_sidecar_addr_env(env_key, value.as_deref(), default_addr)
}

pub(in crate::sidecar_clients::service_manager) fn managed_sidecars_enabled() -> Result<bool, String>
{
    let value = optional_env_string(MANAGED_SIDECARS_ENV)?;
    parse_managed_sidecars_env(value.as_deref())
}

pub(in crate::sidecar_clients::service_manager) fn parse_sidecar_addr_env(
    env_key: &str,
    value: Option<&str>,
    default_addr: &str,
) -> Result<String, String> {
    let value = value.unwrap_or(default_addr).trim();
    if value.is_empty() {
        return Err(format!("{env_key} 不能为空"));
    }
    value
        .parse::<std::net::SocketAddr>()
        .map(|addr| addr.to_string())
        .map_err(|error| format!("{env_key} 必须是有效监听地址: {error}"))
}

pub(in crate::sidecar_clients::service_manager) fn parse_sidecar_url_env(
    env_key: &str,
    value: Option<String>,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().trim_end_matches('/').to_string();
    if value.is_empty() {
        return Err(format!("{env_key} 不能为空"));
    }
    let url = reqwest::Url::parse(&value).map_err(|error| format!("{env_key} 无效: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(format!("{env_key} 必须是 http:// 或 https:// URL"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!("{env_key} 不能包含用户名或密码"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(format!("{env_key} 不能包含查询参数或 fragment"));
    }
    Ok(Some(value))
}

fn optional_env_string(key: &str) -> Result<Option<String>, String> {
    match std::env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("{key} 无法读取: {error}")),
    }
}

pub(in crate::sidecar_clients::service_manager) fn parse_managed_sidecars_env(
    value: Option<&str>,
) -> Result<bool, String> {
    let Some(value) = value else {
        return Ok(false);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!(
            "{MANAGED_SIDECARS_ENV} 必须是 1/0、true/false、yes/no 或 on/off"
        )),
    }
}
