use std::path::PathBuf;
use std::sync::OnceLock;

use parking_lot::RwLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{EnvFilter, Registry, fmt, prelude::*};

const DEFAULT_LOG_FILTER: &str = "info,a4n=debug,sqlx=warn,hyper=warn";

/// 写入文件的非阻塞 guard，必须在程序整个生命周期内保持存活。
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
/// 当前日志根目录，供命令侧读取。
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

type ReloadHandle = Handle<EnvFilter, Registry>;
static FILTER_HANDLE: OnceLock<RwLock<Option<ReloadHandle>>> = OnceLock::new();

/// 初始化 stdout、滚动文件和可热更新的 tracing subscriber。
pub fn init(log_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = rolling::daily(&log_dir, "a4n.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = env_filter_from_environment()?;
    let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(env_filter);

    let stdout_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .compact();
    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true)
        .with_file(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(stdout_layer)
        .with(file_layer)
        .try_init()
        .map_err(|error| std::io::Error::other(format!("初始化日志 subscriber 失败：{error}")))?;

    FILE_GUARD
        .set(guard)
        .map_err(|_| std::io::Error::other("日志文件 guard 已初始化"))?;
    LOG_DIR
        .set(log_dir.clone())
        .map_err(|_| std::io::Error::other("日志目录已初始化"))?;
    FILTER_HANDLE
        .set(RwLock::new(Some(reload_handle)))
        .map_err(|_| std::io::Error::other("日志 reload handle 已初始化"))?;

    install_panic_hook();
    tracing::info!(target: "a4n::logging", log_dir = ?log_dir, "日志系统初始化完成");
    Ok(())
}

pub(super) fn log_dir() -> Option<&'static PathBuf> {
    LOG_DIR.get()
}

pub(super) fn reload_filter(new_filter: EnvFilter) -> Result<(), String> {
    let handle_lock = FILTER_HANDLE
        .get()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    let handle_guard = handle_lock.read();
    let handle = handle_guard
        .as_ref()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    handle
        .reload(new_filter)
        .map_err(|e| format!("应用新级别失败: {e}"))
}

fn env_filter_from_environment() -> Result<EnvFilter, std::io::Error> {
    match std::env::var("RUST_LOG") {
        Ok(value) => build_env_filter(Some(&value)),
        Err(std::env::VarError::NotPresent) => build_env_filter(None),
        Err(err) => Err(invalid_log_filter(format!("读取 RUST_LOG 失败：{err}"))),
    }
}

fn build_env_filter(raw: Option<&str>) -> Result<EnvFilter, std::io::Error> {
    let (label, filter) = match raw {
        Some(value) => ("RUST_LOG", value.trim()),
        None => ("默认日志过滤表达式", DEFAULT_LOG_FILTER),
    };
    if filter.is_empty() {
        return Err(invalid_log_filter(format!("{label} 不能为空")));
    }
    EnvFilter::try_new(filter).map_err(|err| invalid_log_filter(format!("{label} 非法：{err}")))
}

fn invalid_log_filter(message: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
}

fn install_panic_hook() {
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");
        tracing::error!(
            target: "a4n::panic",
            location = %location,
            payload = %payload,
            backtrace = ?std::backtrace::Backtrace::force_capture(),
            "Rust 线程发生 panic"
        );
        prev_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_filter_uses_default_only_when_rust_log_is_missing() {
        let filter = build_env_filter(None).expect("default filter must parse");
        let rendered = filter.to_string();
        for directive in ["info", "a4n=debug", "sqlx=warn", "hyper=warn"] {
            assert!(
                rendered.split(',').any(|part| part == directive),
                "default filter should contain {directive}, got {rendered}"
            );
        }
    }

    #[test]
    fn log_filter_rejects_empty_rust_log() {
        let err = build_env_filter(Some("  ")).unwrap_err();
        assert!(err.to_string().contains("RUST_LOG 不能为空"));
    }

    #[test]
    fn log_filter_rejects_invalid_rust_log() {
        let err = build_env_filter(Some("a4n=[bad")).unwrap_err();
        assert!(err.to_string().contains("RUST_LOG 非法"));
    }
}
