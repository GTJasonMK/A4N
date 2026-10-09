use super::{
    config::{control_addr, worker_id},
    health,
    host::HttpAgentRuntimeHost,
};
use a4n_agent_application::agent::AgentApplication;
use a4n_agent_runtime::{AgentRuntimeState, engine};
use a4n_db::DbHandle;
use reqwest::Client;
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    a4n_worker_runtime::init_worker_tracing();
    let control_address = control_addr()?;
    let control_base_url = format!("http://{control_address}");
    let worker_address = health::bind_addr(control_address)?;
    let worker_id = worker_id()?;
    let worker_token = a4n_config::required_non_empty_env_string("A4N_AGENT_WORKER_TOKEN")?;
    let database_url = a4n_config::required_non_empty_env_string("A4N_DATABASE_URL")?;
    a4n_config::validate_postgres_database_url("A4N_DATABASE_URL", &database_url)?;
    let database = DbHandle::connect_default(database_url).await?;
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
