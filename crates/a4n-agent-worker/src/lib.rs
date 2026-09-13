use a4n_runtime::host_api::agent_runtime::{
    execute_claimed_worker_run, load_agent_worker_run, maintain_agent_worker,
};
use a4n_runtime::{AppState, RuntimeProfile};
use a4n_storage::paths::AppPaths;
use a4n_worker_contract::{
    AGENT_WORKER_AUTH_HEADER, AgentRunPayload, AgentWorkerClaimRequest, WorkerRequest,
};
use reqwest::{Client, StatusCode};
use std::{env, net::SocketAddr, sync::Arc, time::Duration};

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
    let control_base_url = format!("http://{}", control_addr()?);
    let worker_id = worker_id()?;
    let config = a4n_config::app_config_from_env()?;
    let worker_token = a4n_config::required_non_empty_env_string("A4N_AGENT_WORKER_TOKEN")?;
    let runtime_profile =
        RuntimeProfile::configured_from_env()?.unwrap_or_else(RuntimeProfile::default_for_build);
    AppPaths::new(
        config.data_dir.clone(),
        config.cache_dir.clone(),
        config.temp_dir.clone(),
        config.log_dir.clone(),
    )
    .ensure_dirs()?;
    let runtime = Arc::new(AppState::new_agent_worker_with_profile(config, runtime_profile).await?);
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    install_host_dispatcher(
        &runtime,
        client.clone(),
        control_base_url.clone(),
        worker_token.clone(),
    )?;

    tracing::info!(worker_id = %worker_id, control = %control_base_url, "A4N Agent worker started");
    let mut poll = tokio::time::interval(POLL_INTERVAL);
    let mut maintenance = tokio::time::interval(MAINTENANCE_INTERVAL);
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    maintenance.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut shutdown = Box::pin(shutdown_signal());

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = poll.tick() => {
                if let Err(error) = poll_and_execute(
                    &runtime,
                    &client,
                    &control_base_url,
                    &worker_id,
                    &worker_token,
                ).await {
                    tracing::warn!(error = %error, "A4N Agent worker poll failed");
                }
            }
            _ = maintenance.tick() => maintain_agent_worker(runtime.as_ref()).await,
        }
    }

    runtime
        .shutdown_managed_sidecars()
        .await
        .map_err(anyhow::Error::msg)
}

fn install_host_dispatcher(
    runtime: &Arc<AppState>,
    client: Client,
    control_base_url: String,
    worker_token: String,
) -> anyhow::Result<()> {
    runtime
        .set_agent_host_dispatcher(Arc::new(move |_user_id, _command, _payload, request| {
            let client = client.clone();
            let url = format!("{control_base_url}/internal/agent/command");
            let worker_token = worker_token.clone();
            Box::pin(async move {
                let response = client
                    .post(url)
                    .header(AGENT_WORKER_AUTH_HEADER, worker_token)
                    .json(&request)
                    .send()
                    .await
                    .map_err(|error| format!("Control Agent command 请求失败: {error}"))?;
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .map_err(|error| format!("读取 Control Agent command 回执失败: {error}"))?;
                if !status.is_success() {
                    return Err(format!("Control Agent command 返回 {status}: {body}"));
                }
                serde_json::from_str(&body)
                    .map_err(|error| format!("Control Agent command 回执不是 JSON: {error}"))
            })
        }))
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

async fn poll_and_execute(
    runtime: &Arc<AppState>,
    client: &Client,
    control_base_url: &str,
    worker_id: &str,
    worker_token: &str,
) -> anyhow::Result<()> {
    let response = client
        .post(format!("{control_base_url}/internal/agent/claim"))
        .header(AGENT_WORKER_AUTH_HEADER, worker_token)
        .json(&AgentWorkerClaimRequest {
            worker_id: worker_id.to_string(),
        })
        .send()
        .await?;
    if response.status() == StatusCode::NO_CONTENT {
        return Ok(());
    }
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!(
            "Control Agent claim 返回 {status}: {}",
            response.text().await.unwrap_or_default()
        );
    }
    let request: WorkerRequest<AgentRunPayload> = response.json().await?;
    request
        .validate()
        .map_err(|error| anyhow::anyhow!("Control Agent claim contract 无效: {error}"))?;
    request
        .payload
        .validate()
        .map_err(|error| anyhow::anyhow!("Control Agent claim payload 无效: {error}"))?;
    let run_id = request.payload.run_id;
    let claimed =
        load_agent_worker_run(runtime.as_ref(), run_id, &request.lease.fence_token).await?;
    execute_claimed_worker_run(runtime.as_ref(), claimed, request).await?;
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
    let address = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|error| anyhow::anyhow!("默认 Control 地址无效: {error}"))?;
    ensure_nonzero_port(address)
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
