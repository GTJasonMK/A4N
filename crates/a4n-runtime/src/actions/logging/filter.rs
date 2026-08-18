use a4n_core::config::env::optional_non_empty_env_string;
use std::sync::OnceLock;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;
use tracing_subscriber::reload::Handle;

pub struct LogFilterReloader {
    handle: OnceLock<Handle<EnvFilter, Registry>>,
}

impl LogFilterReloader {
    pub const fn new() -> Self {
        Self {
            handle: OnceLock::new(),
        }
    }

    pub fn install(&self, handle: Handle<EnvFilter, Registry>) -> Result<(), String> {
        self.handle
            .set(handle)
            .map_err(|_| "日志 reload handle 已初始化".to_string())
    }

    pub fn reload(&self, filter: &str) -> Result<(), String> {
        let handle = self
            .handle
            .get()
            .ok_or_else(|| "日志系统尚未初始化".to_string())?;
        handle
            .reload(build_log_filter(filter)?)
            .map_err(|error| format!("应用新级别失败: {error}"))
    }
}

pub fn build_log_filter_from_env(default_filter: &str) -> Result<EnvFilter, String> {
    let configured =
        optional_non_empty_env_string("RUST_LOG").map_err(|error| error.to_string())?;
    build_log_filter(configured.as_deref().unwrap_or(default_filter))
}

pub fn build_log_filter(filter: &str) -> Result<EnvFilter, String> {
    let filter = filter.trim();
    if filter.is_empty() {
        return Err("日志过滤表达式不能为空".to_string());
    }
    EnvFilter::try_new(filter).map_err(|err| format!("非法过滤表达式: {err}"))
}
