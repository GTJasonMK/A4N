//! 应用日志系统
//!
//! 提供：
//! - 多 layer subscriber：stdout（开发模式）+ 滚动文件（每天一个 a4n.log）
//! - EnvFilter：支持 `RUST_LOG=a4n=debug,info` 自定义级别
//! - panic hook：把 Rust panic 转成 tracing::error，避免崩溃信息丢失
//! - Tauri 命令：暴露日志目录、读取最近日志、运行时改级别、前端转发
//!
//! 落盘路径：`{config.log_dir}/a4n.log.YYYY-MM-DD`（每天滚动）

use std::path::PathBuf;
use std::sync::OnceLock;

use parking_lot::RwLock;
use serde::Deserialize;
use tauri::State;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{EnvFilter, Registry, fmt, prelude::*};

const DEFAULT_LOG_FILTER: &str = "info,a4n=debug,sqlx=warn,hyper=warn";
pub use a4n_runtime::actions::logging::{LogReadResult, ReadLogsInput, SetLogLevelInput};

/// 写入文件的非阻塞 guard，必须在程序整个生命周期内保持存活
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// 当前的日志根目录（供命令读取）
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// EnvFilter reload handle —— 支持运行时改级别
type ReloadHandle = Handle<EnvFilter, Registry>;
static FILTER_HANDLE: OnceLock<RwLock<Option<ReloadHandle>>> = OnceLock::new();

/// 初始化日志系统
///
/// - `log_dir`：日志文件目录（通常 `AppConfig.log_dir`）
/// - 日志级别从 `RUST_LOG` 读取；缺失时使用项目默认过滤表达式
/// - 返回 `WorkerGuard` 必须长生命周期存活，否则非阻塞 writer 会丢日志；
///   调用方应忽略返回值（已经存进 `FILE_GUARD` 全局静态）
pub fn init(log_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // 确保日志目录存在
    std::fs::create_dir_all(&log_dir)?;

    // 滚动文件 appender：按天滚动，文件名 a4n.log.YYYY-MM-DD
    let file_appender = rolling::daily(&log_dir, "a4n.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = env_filter_from_environment()?;
    let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(env_filter);

    // stdout layer：彩色，紧凑
    let stdout_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .compact();

    // 文件 layer：无色，含完整时间戳，便于 grep
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
        .init();

    // 保存全局引用
    let _ = FILE_GUARD.set(guard);
    let _ = LOG_DIR.set(log_dir.clone());
    let _ = FILTER_HANDLE
        .get_or_init(|| RwLock::new(None))
        .write()
        .replace(reload_handle);

    // 安装 panic hook
    install_panic_hook();

    tracing::info!(target: "a4n::logging", log_dir = ?log_dir, "日志系统初始化完成");
    Ok(())
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

/// 捕获 panic 转 tracing::error
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

// ==================== Tauri 命令 ====================

#[tauri::command]
pub async fn get_log_dir(
    state: State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
) -> Result<String, String> {
    let log_dir = LOG_DIR
        .get()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    a4n_runtime::actions::logging::log_dir_path_for_admin(&state, caller_user_id, log_dir)
        .await
        .map_err(|err| err.to_string())
}

/// 读取日志文件的最后 N 行
#[tauri::command]
pub async fn read_recent_logs(
    state: State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
    input: ReadLogsInput,
) -> Result<LogReadResult, String> {
    let dir = LOG_DIR
        .get()
        .ok_or_else(|| "日志系统尚未初始化".to_string())?;
    a4n_runtime::actions::logging::read_recent_logs_for_admin(&state, caller_user_id, dir, input)
        .await
        .map_err(|err| err.to_string())
}

/// 运行时调整日志级别（不需要重启）
#[tauri::command]
pub async fn set_log_level(
    state: State<'_, a4n_runtime::AppState>,
    caller_user_id: i64,
    input: SetLogLevelInput,
) -> Result<(), String> {
    a4n_runtime::actions::logging::ensure_log_diagnostics_admin(&state, caller_user_id)
        .await
        .map_err(|err| err.to_string())?;
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
        .map_err(|e| format!("应用新级别失败: {e}"))?;
    tracing::info!(target: "a4n::logging", filter = %input.filter, "日志级别已更新");
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct FrontendLogInput {
    /// 级别：debug / info / warn / error
    pub level: String,
    /// 模块标签（如 apiInvoke / AuthStore），用于过滤
    pub target: Option<String>,
    /// 日志正文
    pub message: String,
    /// 可选结构化字段（序列化为 JSON 写入）
    pub fields: Option<serde_json::Value>,
}

/// 接收前端转发的日志，写入与后端相同的 tracing 通道
#[tauri::command]
pub fn log_from_frontend(input: FrontendLogInput) -> Result<(), String> {
    let target = input.target.unwrap_or_else(|| "frontend".to_string());
    let fields_str = match input.fields.as_ref() {
        Some(fields) => fields.to_string(),
        None => String::new(),
    };

    match input.level.to_lowercase().as_str() {
        "trace" => {
            tracing::trace!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "debug" => {
            tracing::debug!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "info" => {
            tracing::info!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "warn" => {
            tracing::warn!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        "error" => {
            tracing::error!(target: "a4n::frontend", source = %target, fields = %fields_str, "{}", input.message)
        }
        other => return Err(format!("未知日志级别: {other}")),
    }
    Ok(())
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
