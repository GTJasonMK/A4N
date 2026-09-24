use super::*;

pub(crate) fn wait_for_control(child: &mut Child, address: SocketAddr) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut last_error = None;
    while started.elapsed() < CONTROL_READY_TIMEOUT {
        if let Some(status) = child.try_wait()? {
            anyhow::bail!(
                "A4N Control 在 listener 就绪前退出: {status}; 最后一次连接错误: {}",
                last_error
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "无".to_string())
            );
        }
        match TcpStream::connect_timeout(&address, CONTROL_READY_CONNECT_TIMEOUT) {
            Ok(_) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        std::thread::sleep(CONTROL_READY_RETRY_INTERVAL);
    }

    anyhow::bail!(
        "A4N Control 在 {} 秒内未监听 {address}; 最后一次连接错误: {}",
        CONTROL_READY_TIMEOUT.as_secs(),
        last_error
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "无".to_string())
    )
}

pub(crate) fn wait_for_managed_worker(
    child: &mut Child,
    config: &ManagedWorkerConfig,
) -> anyhow::Result<()> {
    let required = config
        .metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        config.address,
        &config.token,
        if config.metadata.name == AGENT_WORKER_CATALOG_NAME {
            AGENT_WORKER_AUTH_HEADER
        } else {
            WORKER_AUTH_HEADER
        },
        (config.metadata.name != AGENT_WORKER_CATALOG_NAME).then_some(&config.metadata.name),
        &required,
        &config.metadata.name,
    )
}

pub(crate) fn wait_for_worker_contract(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
    auth_header: &str,
    expected_worker_id: Option<&str>,
    required_capabilities: &[&str],
    worker_name: &str,
) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(WORKER_READY_CONNECT_TIMEOUT)
        .timeout(WORKER_READY_CONNECT_TIMEOUT)
        .build()
        .map_err(|error| anyhow::anyhow!("初始化 worker health client 失败: {error}"))?;
    let started = Instant::now();
    let health_url = format!("http://{address}{WORKER_HEALTH_PATH}");
    let capabilities_url = format!("http://{address}{WORKER_CAPABILITIES_PATH}");
    let mut last_error = None;
    while started.elapsed() < WORKER_READY_TIMEOUT {
        if let Some(status) = child.try_wait()? {
            anyhow::bail!(
                "{worker_name} 在健康检查通过前退出: {status}; 最后一次错误: {}",
                last_error
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "无".to_string())
            );
        }

        match client
            .get(&health_url)
            .header(auth_header, worker_token)
            .send()
        {
            Ok(response) if response.status().is_success() => {
                let health = response.json::<WorkerHealth>().map_err(|error| {
                    anyhow::anyhow!("{worker_name} health 响应不是有效合同: {error}")
                })?;
                if let Some(expected_worker_id) = expected_worker_id {
                    health.validate_for(expected_worker_id).map_err(|error| {
                        anyhow::anyhow!("{worker_name} health 合同无效: {error}")
                    })?;
                } else {
                    health.validate().map_err(|error| {
                        anyhow::anyhow!("{worker_name} health 合同无效: {error}")
                    })?;
                }
                if health.status == WorkerHealthStatus::Healthy {
                    let capabilities = client
                        .get(&capabilities_url)
                        .header(auth_header, worker_token)
                        .send()
                        .map_err(|error| {
                            anyhow::anyhow!("{worker_name} capabilities 请求失败: {error}")
                        })?
                        .json::<WorkerCapabilities>()
                        .map_err(|error| {
                            anyhow::anyhow!("{worker_name} capabilities 响应不是有效合同: {error}")
                        })?;
                    if let Some(expected_worker_id) = expected_worker_id {
                        capabilities
                            .validate_for(expected_worker_id)
                            .map_err(|error| {
                                anyhow::anyhow!("{worker_name} capabilities 合同无效: {error}")
                            })?;
                    } else {
                        capabilities.validate().map_err(|error| {
                            anyhow::anyhow!("{worker_name} capabilities 合同无效: {error}")
                        })?;
                    }
                    for required in required_capabilities {
                        if !capabilities
                            .capabilities
                            .iter()
                            .any(|capability| capability.enabled && capability.name == *required)
                        {
                            anyhow::bail!("{worker_name} capabilities 缺少已启用操作: {required}");
                        }
                    }
                    return Ok(());
                }
                last_error = Some(anyhow::anyhow!(
                    "{worker_name} health 状态为 {:?}",
                    health.status
                ));
            }
            Ok(response) => {
                last_error = Some(anyhow::anyhow!(
                    "{worker_name} health 返回 {}",
                    response.status()
                ));
            }
            Err(error) => last_error = Some(anyhow::anyhow!(error)),
        }
        std::thread::sleep(WORKER_READY_RETRY_INTERVAL);
    }

    anyhow::bail!(
        "{worker_name} 在 {} 秒内未通过健康检查 {health_url}: {}",
        WORKER_READY_TIMEOUT.as_secs(),
        last_error
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "无".to_string())
    )
}
