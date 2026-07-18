use std::ffi::OsString;
use std::path::Path;
use std::sync::Mutex as StdMutex;

use super::model::download_proxy_env_vars;

pub(crate) static HF_HOME_OVERRIDE_LOCK: StdMutex<()> = StdMutex::new(());

pub(crate) struct ScopedHfHubEnv<'a> {
    _guard: std::sync::MutexGuard<'a, ()>,
    _vars: Vec<ScopedEnvVar>,
}

impl<'a> ScopedHfHubEnv<'a> {
    pub(crate) fn set(cache_dir: &Path) -> Result<Self, String> {
        let guard = HF_HOME_OVERRIDE_LOCK
            .lock()
            .map_err(|_| "fastembed 模型缓存环境锁已损坏".to_string())?;
        let mut vars = Vec::new();

        // fastembed 5.x 在下载时会让 HF_HOME 优先于 with_cache_dir。这里在全局锁
        // 内临时固定 HF_HOME，确保模型写入应用数据目录，而不是用户系统缓存。
        vars.push(ScopedEnvVar::set("HF_HOME", cache_dir.as_os_str()));

        // ureq/hf-hub 会从标准代理环境变量读取代理。这里把桌面系统代理和
        // 显式代理环境统一成 ureq 可稳定使用的 HTTP(S) 下载代理。
        vars.extend(download_proxy_env_vars());

        Ok(Self {
            _guard: guard,
            _vars: vars,
        })
    }
}

pub(crate) struct ScopedEnvVar {
    key: &'static str,
    previous: Option<OsString>,
}

impl ScopedEnvVar {
    pub(super) fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }
}

impl Drop for ScopedEnvVar {
    fn drop(&mut self) {
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
