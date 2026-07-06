use a4n_runtime::actions::logging::SetLogLevelInput;
use parking_lot::RwLock;
use std::sync::OnceLock;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt, util::SubscriberInitExt};

type ReloadHandle = Handle<EnvFilter, Registry>;
static FILTER_HANDLE: OnceLock<RwLock<Option<ReloadHandle>>> = OnceLock::new();

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();
    let _ = FILTER_HANDLE
        .get_or_init(|| RwLock::new(None))
        .write()
        .replace(reload_handle);
}

pub fn set_log_level(input: SetLogLevelInput) -> Result<(), String> {
    let handle_lock = FILTER_HANDLE
        .get()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    let handle_guard = handle_lock.read();
    let handle = handle_guard
        .as_ref()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    let new_filter = a4n_runtime::actions::logging::build_log_filter(&input.filter)?;
    handle
        .reload(new_filter)
        .map_err(|err| format!("应用新级别失败: {err}"))?;
    tracing::info!(target: "a4n::logging", filter = %input.filter, "日志级别已更新");
    Ok(())
}
