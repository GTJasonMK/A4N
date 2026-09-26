mod health;
mod host;

use a4n_agent_application::agent::AgentApplication;
use a4n_agent_runtime::{AgentRuntimeState, engine};
use a4n_db::DbHandle;
use host::HttpAgentRuntimeHost;
use reqwest::Client;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::oneshot;

const WORKER_ID_ENV: &str = "A4N_AGENT_WORKER_ID";

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    init_tracing();
    let control_address = control_addr()?;
    let control_base_url = format!("http://{control_address}");
    let worker_address = health::bind_addr(control_address)?;
    let worker_id = worker_id()?;
    let worker_token = a4n_config::required_non_empty_env_string("A4N_AGENT_WORKER_TOKEN")?;
    let database_url = a4n_config::required_non_empty_env_string("A4N_DATABASE_URL")?;
    a4n_config::validate_postgres_database_url("A4N_DATABASE_URL", &database_url)?;
    let database = DbHandle::connect_with_default_options(database_url).await?;
    let application = Arc::new(AgentApplication::new(database));
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    let host = Arc::new(HttpAgentRuntimeHost::new(
        client,
        control_base_url.clone(),
        worker_token.clone(),
        application.clone(),
    ));
    let runtime = Arc::new(AgentRuntimeState::new(host));
    let listener = tokio::net::TcpListener::bind(worker_address).await?;
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let mut health_server = tokio::spawn(health::serve(
        listener,
        health::router(
            worker_id.clone(),
            worker_token.clone(),
            application.clone(),
            runtime.clone(),
        ),
        shutdown_receiver,
    ));

    tracing::info!(
        worker_id = %worker_id,
        control = %control_base_url,
        health = %worker_address,
        "A4N Agent worker started"
    );
    let mut driver = Box::pin(engine::run_worker(
        runtime.as_ref().clone(),
        runtime.cancel_token(),
        runtime.notify(),
        worker_id,
    ));
    let mut driver_stopped = false;
    let mut shutdown = Box::pin(shutdown_signal());

    let mut health_server_result = None;
    tokio::select! {
        _ = &mut shutdown => {}
        result = &mut health_server => {
            health_server_result = Some(result);
        }
        _ = &mut driver => { driver_stopped = true; }
    }

    runtime.shutdown().await;
    if !driver_stopped {
        driver.await;
    }
    let _ = shutdown_sender.send(());
    let health_result = match health_server_result {
        Some(result) => match result {
            Ok(Ok(())) => Err(anyhow::anyhow!(
                "A4N Agent worker health server stopped unexpectedly"
            )),
            Ok(Err(error)) => Err(error),
            Err(error) => Err(anyhow::anyhow!(
                "A4N Agent worker health server task failed: {error}"
            )),
        },
        None => health_server
            .await
            .map_err(|error| anyhow::anyhow!("A4N Agent worker health server task failed: {error}"))
            .and_then(|result| result),
    };
    if driver_stopped {
        anyhow::bail!("A4N Agent driver stopped unexpectedly");
    }
    health_result
}

fn control_addr() -> anyhow::Result<SocketAddr> {
    if let Some(value) = env::var_os("A4N_CONTROL_ADDR") {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("A4N_CONTROL_ADDR 必须是有效 UTF-8 地址"))?;
        let address = value
            .parse()
            .map_err(|error| anyhow::anyhow!("A4N_CONTROL_ADDR 无效: {error}"))?;
        return ensure_nonzero_port(address);
    }
    let web_addr = env::var("A4N_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("A4N_WEB_ADDR 无效: {error}"))?;
    let port = web_addr
        .port()
        .checked_add(1)
        .filter(|port| *port != 0)
        .unwrap_or(18080);
    ensure_nonzero_port(format!("127.0.0.1:{port}").parse()?)
}

fn ensure_nonzero_port(address: SocketAddr) -> anyhow::Result<SocketAddr> {
    if address.port() == 0 {
        anyhow::bail!("A4N_CONTROL_ADDR 不允许使用端口 0");
    }
    Ok(address)
}

fn worker_id() -> anyhow::Result<String> {
    match a4n_config::optional_non_empty_env_string(WORKER_ID_ENV)? {
        Some(value) => Ok(value),
        None => Ok(format!("a4n-agent:{}", std::process::id())),
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
