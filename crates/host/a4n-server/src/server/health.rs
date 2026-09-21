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

pub(crate) fn wait_for_agent_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = agent_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        AGENT_WORKER_AUTH_HEADER,
        None,
        &required,
        "A4N Agent worker",
    )
}

pub(crate) fn wait_for_novel_planning_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = novel_planning_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(NOVEL_PLANNING_WORKER_CATALOG_NAME),
        &required,
        "A4N Novel Planning worker",
    )
}

pub(crate) fn wait_for_artifact_media_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = artifact_media_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(ARTIFACT_MEDIA_WORKER_CATALOG_NAME),
        &required,
        "A4N Artifact/Media worker",
    )
}

pub(crate) fn wait_for_outline_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = outline_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(OUTLINE_WORKER_CATALOG_NAME),
        &required,
        "A4N Outline worker",
    )
}

pub(crate) fn wait_for_blueprint_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = blueprint_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(BLUEPRINT_WORKER_CATALOG_NAME),
        &required,
        "A4N Blueprint worker",
    )
}

pub(crate) fn wait_for_asset_graph_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = asset_graph_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(ASSET_GRAPH_WORKER_CATALOG_NAME),
        &required,
        "A4N Asset Graph worker",
    )
}

pub(crate) fn wait_for_foreshadowing_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = foreshadowing_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(FORESHADOWING_WORKER_CATALOG_NAME),
        &required,
        "A4N Foreshadowing worker",
    )
}

pub(crate) fn wait_for_object_profile_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = object_profile_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(OBJECT_PROFILE_WORKER_CATALOG_NAME),
        &required,
        "A4N Object Profile worker",
    )
}

pub(crate) fn wait_for_scene_profile_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = scene_profile_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(SCENE_PROFILE_WORKER_CATALOG_NAME),
        &required,
        "A4N Scene Profile worker",
    )
}

pub(crate) fn wait_for_profile_material_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = profile_material_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROFILE_MATERIAL_WORKER_CATALOG_NAME),
        &required,
        "A4N Profile Material worker",
    )
}

pub(crate) fn wait_for_project_status_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_status_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_STATUS_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Status worker",
    )
}

pub(crate) fn wait_for_theme_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = theme_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(THEME_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Theme Read worker",
    )
}

pub(crate) fn wait_for_project_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Read worker",
    )
}

pub(crate) fn wait_for_import_review_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = import_review_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(IMPORT_REVIEW_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Import Review Read worker",
    )
}

pub(crate) fn wait_for_inspiration_history_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = inspiration_history_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(INSPIRATION_HISTORY_WORKER_CATALOG_NAME),
        &required,
        "A4N Inspiration History worker",
    )
}

pub(crate) fn wait_for_project_access_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_access_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_ACCESS_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Access worker",
    )
}

pub(crate) fn wait_for_project_collaboration_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_collaboration_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_COLLABORATION_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Collaboration worker",
    )
}

pub(crate) fn wait_for_project_mutation_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_mutation_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_MUTATION_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Mutation worker",
    )
}

pub(crate) fn wait_for_chapter_scene_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = chapter_scene_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(CHAPTER_SCENE_WORKER_CATALOG_NAME),
        &required,
        "A4N Chapter Scene worker",
    )
}

pub(crate) fn wait_for_chapter_content_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = chapter_content_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(CHAPTER_CONTENT_WORKER_CATALOG_NAME),
        &required,
        "A4N Chapter Content worker",
    )
}

pub(crate) fn wait_for_workflow_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = workflow_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(WORKFLOW_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Workflow Read worker",
    )
}

pub(crate) fn wait_for_config_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = config_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(CONFIG_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Config Read worker",
    )
}

pub(crate) fn wait_for_character_mutation_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = character_mutation_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(CHARACTER_MUTATION_WORKER_CATALOG_NAME),
        &required,
        "A4N Character Mutation worker",
    )
}

pub(crate) fn wait_for_video_record_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = video_record_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(VIDEO_RECORD_WORKER_CATALOG_NAME),
        &required,
        "A4N Video Record worker",
    )
}

pub(crate) fn wait_for_manga_record_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = manga_record_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(MANGA_RECORD_WORKER_CATALOG_NAME),
        &required,
        "A4N Manga Record worker",
    )
}

pub(crate) fn wait_for_portrait_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = portrait_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PORTRAIT_WORKER_CATALOG_NAME),
        &required,
        "A4N Portrait worker",
    )
}

pub(crate) fn wait_for_source_snapshot_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = source_snapshot_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(SOURCE_SNAPSHOT_WORKER_CATALOG_NAME),
        &required,
        "A4N Source Snapshot worker",
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
