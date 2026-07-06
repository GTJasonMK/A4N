use super::env::ScopedEnvVar;
use super::system::read_system_proxy;

const NO_PROXY_ENV_KEYS: [&str; 2] = ["NO_PROXY", "no_proxy"];

#[derive(Debug, Clone, Default)]
pub(in crate::embedding::fastembed) struct SystemProxy {
    pub(in crate::embedding::fastembed) http_proxy: Option<String>,
    pub(in crate::embedding::fastembed) https_proxy: Option<String>,
    pub(in crate::embedding::fastembed) all_proxy: Option<String>,
    pub(in crate::embedding::fastembed) no_proxy: Option<String>,
}

impl SystemProxy {
    pub(super) fn is_empty(&self) -> bool {
        self.http_proxy.is_none() && self.https_proxy.is_none() && self.all_proxy.is_none()
    }
}

pub(super) fn download_proxy_env_vars() -> Vec<ScopedEnvVar> {
    let proxy = merge_download_proxy(explicit_proxy_env(), read_system_proxy());
    proxy.map(proxy_to_env_vars).unwrap_or_default()
}

pub(in crate::embedding::fastembed) fn merge_download_proxy(
    explicit: Option<SystemProxy>,
    system: Option<SystemProxy>,
) -> Option<SystemProxy> {
    let explicit = explicit.unwrap_or_default();
    let system = system.unwrap_or_default();

    let http_proxy = explicit
        .http_proxy
        .clone()
        .or_else(|| system.http_proxy.clone())
        .or_else(|| explicit.https_proxy.clone())
        .or_else(|| system.https_proxy.clone());
    let https_proxy = explicit
        .https_proxy
        .clone()
        .or_else(|| system.https_proxy.clone())
        .or_else(|| explicit.http_proxy.clone())
        .or_else(|| system.http_proxy.clone());
    let all_proxy = https_proxy
        .clone()
        .or_else(|| http_proxy.clone())
        .or_else(|| explicit.all_proxy.clone())
        .or_else(|| system.all_proxy.clone());

    let proxy = SystemProxy {
        http_proxy,
        https_proxy,
        all_proxy,
        no_proxy: explicit.no_proxy.or(system.no_proxy),
    };
    (!proxy.is_empty()).then_some(proxy)
}

pub(in crate::embedding::fastembed) fn proxy_to_env_vars(proxy: SystemProxy) -> Vec<ScopedEnvVar> {
    let mut vars = Vec::new();

    let https_proxy = proxy
        .https_proxy
        .clone()
        .or_else(|| proxy.http_proxy.clone())
        .or_else(|| proxy.all_proxy.clone());
    let http_proxy = proxy
        .http_proxy
        .clone()
        .or_else(|| proxy.https_proxy.clone())
        .or_else(|| proxy.all_proxy.clone());
    let all_proxy = https_proxy.clone().or_else(|| proxy.all_proxy.clone());

    if let Some(value) = http_proxy {
        vars.push(ScopedEnvVar::set("HTTP_PROXY", &value));
        vars.push(ScopedEnvVar::set("http_proxy", &value));
    }
    if let Some(value) = https_proxy {
        vars.push(ScopedEnvVar::set("HTTPS_PROXY", &value));
        vars.push(ScopedEnvVar::set("https_proxy", &value));
    }
    if let Some(value) = all_proxy {
        vars.push(ScopedEnvVar::set("ALL_PROXY", &value));
        vars.push(ScopedEnvVar::set("all_proxy", &value));
    }
    if !has_non_empty_env(&NO_PROXY_ENV_KEYS)
        && let Some(value) = proxy.no_proxy
    {
        vars.push(ScopedEnvVar::set("NO_PROXY", &value));
        vars.push(ScopedEnvVar::set("no_proxy", &value));
    }
    vars
}

fn explicit_proxy_env() -> Option<SystemProxy> {
    let proxy = SystemProxy {
        http_proxy: first_non_empty_env(&["HTTP_PROXY", "http_proxy"]),
        https_proxy: first_non_empty_env(&["HTTPS_PROXY", "https_proxy"]),
        all_proxy: first_non_empty_env(&["ALL_PROXY", "all_proxy"]),
        no_proxy: first_non_empty_env(&NO_PROXY_ENV_KEYS),
    };
    (!proxy.is_empty()).then_some(proxy)
}

fn first_non_empty_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn has_non_empty_env(keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        std::env::var_os(key).is_some_and(|value| !value.to_string_lossy().trim().is_empty())
    })
}
