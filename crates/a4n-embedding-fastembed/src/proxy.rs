mod env;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod model;
mod system;
mod url;
#[cfg(target_os = "windows")]
mod windows;

pub(crate) use env::ScopedHfHubEnv;
#[cfg(test)]
pub(crate) use env::{HF_HOME_OVERRIDE_LOCK, ScopedEnvVar};
#[cfg(all(test, target_os = "linux"))]
pub(crate) use linux::{parse_gsettings_array, parse_gsettings_string};
#[cfg(test)]
pub(crate) use model::{SystemProxy, proxy_to_env_vars};
#[cfg(test)]
pub(crate) use url::proxy_url;
