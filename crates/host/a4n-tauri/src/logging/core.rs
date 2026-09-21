use std::path::PathBuf;
use std::sync::OnceLock;

use a4n_runtime::host_api::logging::LogFilterReloader;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*};

const DEFAULT_LOG_FILTER: &str = "info,a4n=debug,sqlx=warn,hyper=warn";

/// 写入文件的非阻塞 guard，必须在程序整个生命周期内保持存活。
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
/// 当前日志根目录，供命令侧读取。
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub(super) static FILTER_RELOADER: LogFilterReloader = LogFilterReloader::new();

/// 初始化 stdout、滚动文件和可热更新的 tracing subscriber。
pub fn init(log_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = rolling::daily(&log_dir, "a4n.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = a4n_runtime::host_api::logging::build_log_filter_from_env(DEFAULT_LOG_FILTER)
        .map_err(invalid_log_filter)?;
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
    FILTER_RELOADER
        .install(reload_handle)
        .map_err(std::io::Error::other)?;

    install_panic_hook();
    tracing::info!(target: "a4n::logging", log_dir = ?log_dir, "日志系统初始化完成");
    Ok(())
}

pub(super) fn log_dir() -> Option<&'static PathBuf> {
    LOG_DIR.get()
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
