pub fn init_worker_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

pub async fn wait_for_ctrl_c() {
    let _ = tokio::signal::ctrl_c().await;
}
