use tracing_subscriber::EnvFilter;

pub fn build_log_filter(filter: &str) -> Result<EnvFilter, String> {
    let filter = filter.trim();
    if filter.is_empty() {
        return Err("日志过滤表达式不能为空".to_string());
    }
    EnvFilter::try_new(filter).map_err(|err| format!("非法过滤表达式: {err}"))
}
