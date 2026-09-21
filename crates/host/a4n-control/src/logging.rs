use a4n_runtime::host_api::logging::{LogFilterReloader, SetLogLevelInput};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static FILTER_RELOADER: LogFilterReloader = LogFilterReloader::new();

const DEFAULT_LOG_FILTER: &str = "info,sqlx=warn,hyper=warn";

pub fn init_tracing() -> anyhow::Result<()> {
    let filter = a4n_runtime::host_api::logging::build_log_filter_from_env(DEFAULT_LOG_FILTER)
        .map_err(anyhow::Error::msg)?;
    let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;
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
