mod health;
mod host;

use a4n_agent_application::agent::AgentApplication;
use a4n_agent_runtime::{AgentRuntimeState, engine};
use a4n_db::DbHandle;
use host::HttpAgentRuntimeHost;
use reqwest::Client;
use sea_orm::Database;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::oneshot;

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(2);
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
    let database = Database::connect(database_url).await?;
    let application = Arc::new(AgentApplication::new(DbHandle::new(Arc::new(database))));
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    let host = Arc::new(
        HttpAgentRuntimeHost::new(client, control_base_url.clone(), worker_token.clone())
            .with_application(application),
    );
    let runtime = Arc::new(AgentRuntimeState::new(host));
    let listener = tokio::net::TcpListener::bind(worker_address).await?;
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let mut health_server = tokio::spawn(health::serve(
        listener,
        health::router(worker_id.clone(), worker_token.clone()),
        shutdown_receiver,
    ));

    tracing::info!(
        worker_id = %worker_id,
        control = %control_base_url,
        health = %worker_address,
        "A4N Agent worker started"
    );
    let mut poll = tokio::time::interval(POLL_INTERVAL);
    let mut maintenance = tokio::time::interval(MAINTENANCE_INTERVAL);
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    maintenance.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut shutdown = Box::pin(shutdown_signal());

    let mut health_server_result = None;
    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            result = &mut health_server => {
                health_server_result = Some(result);
                break;
            }
            _ = poll.tick() => {
                if let Err(error) = poll_and_execute(
                    &runtime,
                    &worker_id,
                ).await {
                    tracing::warn!(error = %error, "A4N Agent worker poll failed");
                }
            }
            _ = maintenance.tick() => engine::maintain_agent_worker(runtime.as_ref()).await,
        }
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
    runtime.shutdown().await;
    health_result
}

async fn poll_and_execute(runtime: &Arc<AgentRuntimeState>, worker_id: &str) -> anyhow::Result<()> {
    let Some(request) = engine::claim_agent_worker_request(runtime.as_ref(), worker_id).await?
    else {
        return Ok(());
    };
    let claimed = engine::agent_worker_run_from_wire(request.payload.worker_run.clone())?;
    engine::execute_claimed_worker_run(runtime.as_ref(), claimed, request).await?;
    Ok(())
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
