use a4n_runtime::host_api::logging::SetLogLevelInput;
use parking_lot::RwLock;
use std::sync::OnceLock;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt, util::SubscriberInitExt};

type ReloadHandle = Handle<EnvFilter, Registry>;
static FILTER_HANDLE: OnceLock<RwLock<Option<ReloadHandle>>> = OnceLock::new();

const DEFAULT_LOG_FILTER: &str = "info";

pub fn init_tracing() -> anyhow::Result<()> {
    let filter = env_filter_from_environment().map_err(anyhow::Error::msg)?;
    let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;
    FILTER_HANDLE
        .set(RwLock::new(Some(reload_handle)))
        .map_err(|_| anyhow::anyhow!("日志 reload handle 已初始化"))?;
    Ok(())
}

fn env_filter_from_environment() -> Result<EnvFilter, String> {
    match std::env::var("RUST_LOG") {
        Ok(value) => initial_log_filter(Some(&value)),
        Err(std::env::VarError::NotPresent) => initial_log_filter(None),
        Err(err) => Err(format!("读取 RUST_LOG 失败: {err}")),
    }
}

fn initial_log_filter(raw: Option<&str>) -> Result<EnvFilter, String> {
    a4n_runtime::host_api::logging::build_log_filter(raw.unwrap_or(DEFAULT_LOG_FILTER))
}

pub fn set_log_level(input: SetLogLevelInput) -> Result<(), String> {
    let handle_lock = FILTER_HANDLE
        .get()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    let handle_guard = handle_lock.read();
    let handle = handle_guard
        .as_ref()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    let new_filter = a4n_runtime::host_api::logging::build_log_filter(&input.filter)?;
    handle
        .reload(new_filter)
        .map_err(|err| format!("应用新级别失败: {err}"))?;
    tracing::info!(target: "a4n::logging", filter = %input.filter, "日志级别已更新");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_log_filter_uses_default_only_when_rust_log_is_missing() {
        initial_log_filter(None).expect("missing RUST_LOG should use the default filter");
    }

    #[test]
    fn initial_log_filter_rejects_blank_rust_log() {
        let error = initial_log_filter(Some("  ")).expect_err("blank RUST_LOG must fail");
        assert!(error.contains("不能为空"));
    }

    #[test]
    fn initial_log_filter_rejects_invalid_rust_log() {
        let error = initial_log_filter(Some("a4n==debug")).expect_err("invalid RUST_LOG must fail");
        assert!(error.contains("非法过滤表达式"));
    }
}
