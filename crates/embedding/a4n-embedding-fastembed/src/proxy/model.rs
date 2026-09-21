use super::env::ScopedEnvVar;
use super::system::read_system_proxy;

const HTTP_PROXY_ENV_KEYS: [&str; 2] = ["HTTP_PROXY", "http_proxy"];
const HTTPS_PROXY_ENV_KEYS: [&str; 2] = ["HTTPS_PROXY", "https_proxy"];
const ALL_PROXY_ENV_KEYS: [&str; 2] = ["ALL_PROXY", "all_proxy"];
const NO_PROXY_ENV_KEYS: [&str; 2] = ["NO_PROXY", "no_proxy"];

#[derive(Default)]
pub(crate) struct SystemProxy {
    pub(crate) http_proxy: Option<String>,
    pub(crate) https_proxy: Option<String>,
    pub(crate) all_proxy: Option<String>,
    pub(crate) no_proxy: Option<String>,
}

impl SystemProxy {
    pub(super) fn is_empty(&self) -> bool {
        self.http_proxy.is_none() && self.https_proxy.is_none() && self.all_proxy.is_none()
    }
}

pub(super) fn download_proxy_env_vars() -> Vec<ScopedEnvVar> {
    let Some(proxy) = read_system_proxy() else {
        return Vec::new();
    };
    proxy_to_env_vars(proxy)
}

pub(crate) fn proxy_to_env_vars(proxy: SystemProxy) -> Vec<ScopedEnvVar> {
    let mut vars = Vec::new();
    set_missing_proxy_env(&mut vars, HTTP_PROXY_ENV_KEYS, proxy.http_proxy);
    set_missing_proxy_env(&mut vars, HTTPS_PROXY_ENV_KEYS, proxy.https_proxy);
    set_missing_proxy_env(&mut vars, ALL_PROXY_ENV_KEYS, proxy.all_proxy);
    set_missing_proxy_env(&mut vars, NO_PROXY_ENV_KEYS, proxy.no_proxy);
    vars
}

fn set_missing_proxy_env(
    vars: &mut Vec<ScopedEnvVar>,
    keys: [&'static str; 2],
    value: Option<String>,
) {
    if has_non_empty_env(&keys) {
        return;
    }
    let Some(value) = value else {
        return;
    };
    for key in keys {
        vars.push(ScopedEnvVar::set(key, &value));
    }
}

fn has_non_empty_env(keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        std::env::var_os(key).is_some_and(|value| !value.to_string_lossy().trim().is_empty())
    })
}
