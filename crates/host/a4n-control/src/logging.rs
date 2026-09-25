use a4n_runtime::host_api::logging::{LOG_FILE_PREFIX, LogFilterReloader, SetLogLevelInput};
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static FILTER_RELOADER: LogFilterReloader = LogFilterReloader::new();
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

const DEFAULT_LOG_FILTER: &str = "info,sqlx=warn,hyper=warn";

pub fn init_tracing() -> anyhow::Result<()> {
    let log_dir = a4n_config::app_config_from_env()?.log_dir;
    std::fs::create_dir_all(&log_dir)?;
    let appender = tracing_appender::rolling::daily(&log_dir, LOG_FILE_PREFIX);
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = a4n_runtime::host_api::logging::build_log_filter_from_env(DEFAULT_LOG_FILTER)
        .map_err(anyhow::Error::msg)?;
    let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .try_init()?;
    FILE_GUARD
        .set(guard)
        .map_err(|_| anyhow::anyhow!("日志文件 guard 已初始化"))?;
    FILTER_RELOADER
        .install(reload_handle)
        .map_err(anyhow::Error::msg)?;
    Ok(())
}

pub fn set_log_level(input: SetLogLevelInput) -> Result<(), String> {
    FILTER_RELOADER.reload(&input.filter)?;
    tracing::info!(target: "a4n::logging", filter = %input.filter, "日志级别已更新");
    Ok(())
}
