use a4n_config::optional_non_empty_env_string;
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

    pub fn current_filter(&self) -> Result<String, String> {
        let handle = self
            .handle
            .get()
            .ok_or_else(|| "日志系统尚未初始化".to_string())?;
        handle
            .with_current(|filter| filter.to_string())
            .map_err(|error| format!("读取当前日志过滤器失败: {error}"))
    }
}

impl Default for LogFilterReloader {
    fn default() -> Self {
        Self::new()
    }
}

pub fn build_log_filter_from_env(default_filter: &str) -> Result<EnvFilter, String> {
    let configured =
        optional_non_empty_env_string("RUST_LOG").map_err(|error| error.to_string())?;
    build_log_filter(configured.as_deref().unwrap_or(default_filter))
}

pub(crate) fn build_log_filter(filter: &str) -> Result<EnvFilter, String> {
    let filter = filter.trim();
    if filter.is_empty() {
        return Err("日志过滤表达式不能为空".to_string());
    }
    EnvFilter::try_new(filter).map_err(|err| format!("非法过滤表达式: {err}"))
}

#[cfg(test)]
mod tests {
    use super::LogFilterReloader;
    use tracing_subscriber::prelude::*;

    #[test]
    fn current_filter_reads_the_active_filter_after_reload() {
        let (layer, handle) = tracing_subscriber::reload::Layer::new(
            super::build_log_filter("warn").expect("initial filter is valid"),
        );
        let subscriber = tracing_subscriber::registry().with(layer);
        let _default_subscriber = tracing::subscriber::set_default(subscriber);
        let reloader = LogFilterReloader::new();
        reloader.install(handle).expect("install reload handle");

        assert_eq!(
            reloader.current_filter().expect("read active filter"),
            "warn"
        );
        reloader.reload("error").expect("reload filter");
        assert_eq!(
            reloader.current_filter().expect("read reloaded filter"),
            "error"
        );
    }
}
