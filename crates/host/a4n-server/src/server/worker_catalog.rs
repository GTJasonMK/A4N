use super::*;

pub(crate) fn binary_fingerprint(path: &Path) -> anyhow::Result<BinaryFingerprint> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| anyhow::anyhow!("读取 worker 二进制 {} 失败: {error}", path.display()))?;
    let modified = metadata.modified().map_err(|error| {
        anyhow::anyhow!(
            "读取 worker 二进制 {} 修改时间失败: {error}",
            path.display()
        )
    })?;
    Ok(BinaryFingerprint {
        modified,
        length: metadata.len(),
    })
}

pub(crate) fn optional_binary_fingerprint(
    path: Option<&Path>,
) -> anyhow::Result<Option<BinaryFingerprint>> {
    let Some(path) = path else {
        return Ok(None);
    };
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(BinaryFingerprint {
            modified: metadata.modified().map_err(|error| {
                anyhow::anyhow!(
                    "读取 reload marker {} 修改时间失败: {error}",
                    path.display()
                )
            })?,
            length: metadata.len(),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(anyhow::anyhow!(
            "读取 reload marker {} 失败: {error}",
            path.display()
        )),
    }
}

pub(crate) fn web_addr() -> anyhow::Result<SocketAddr> {
    env::var("A4N_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("A4N_WEB_ADDR 无效: {error}"))
}

pub(crate) fn control_addr() -> anyhow::Result<SocketAddr> {
    if let Some(value) = env::var_os("A4N_CONTROL_ADDR") {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("A4N_CONTROL_ADDR 必须是有效 UTF-8 地址"))?;
        let address = value
            .parse()
            .map_err(|error| anyhow::anyhow!("A4N_CONTROL_ADDR 无效: {error}"))?;
        ensure_nonzero_port("A4N_CONTROL_ADDR", address)
    } else {
        control_addr_from_web(web_addr()?)
    }
}

pub(crate) fn control_addr_from_web(web_addr: SocketAddr) -> anyhow::Result<SocketAddr> {
    if web_addr.port() == 0 {
        return Ok(SocketAddr::from(([127, 0, 0, 1], 18080)));
    }
    let control_port = web_addr
        .port()
        .checked_add(1)
        .filter(|port| *port != 0)
        .unwrap_or(18080);
    Ok(SocketAddr::from(([127, 0, 0, 1], control_port)))
}

pub(crate) fn agent_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = agent_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn agent_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(AGENT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            AGENT_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn novel_planning_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(NOVEL_PLANNING_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            NOVEL_PLANNING_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn novel_planning_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = novel_planning_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn artifact_media_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(ARTIFACT_MEDIA_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            ARTIFACT_MEDIA_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn artifact_media_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = artifact_media_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn outline_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(OUTLINE_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            OUTLINE_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn outline_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = outline_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn blueprint_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(BLUEPRINT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            BLUEPRINT_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn blueprint_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = blueprint_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn asset_graph_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(ASSET_GRAPH_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            ASSET_GRAPH_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn asset_graph_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = asset_graph_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn foreshadowing_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(FORESHADOWING_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            FORESHADOWING_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn foreshadowing_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = foreshadowing_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn object_profile_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(OBJECT_PROFILE_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            OBJECT_PROFILE_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn object_profile_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = object_profile_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn scene_profile_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(SCENE_PROFILE_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            SCENE_PROFILE_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn scene_profile_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = scene_profile_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn profile_material_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROFILE_MATERIAL_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROFILE_MATERIAL_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn profile_material_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = profile_material_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn project_status_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_STATUS_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_STATUS_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn project_status_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_status_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn portrait_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PORTRAIT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PORTRAIT_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn portrait_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = portrait_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn source_snapshot_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(SOURCE_SNAPSHOT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            SOURCE_SNAPSHOT_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn source_snapshot_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = source_snapshot_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn theme_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(THEME_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            THEME_READ_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn theme_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = theme_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn project_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_READ_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn project_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn import_review_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(IMPORT_REVIEW_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            IMPORT_REVIEW_READ_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn import_review_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = import_review_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn inspiration_history_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(INSPIRATION_HISTORY_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            INSPIRATION_HISTORY_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn inspiration_history_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = inspiration_history_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn project_access_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_ACCESS_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_ACCESS_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn project_access_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_access_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn project_collaboration_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_COLLABORATION_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_COLLABORATION_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn project_collaboration_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_collaboration_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn project_mutation_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_MUTATION_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_MUTATION_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn project_mutation_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_mutation_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn chapter_scene_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(CHAPTER_SCENE_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            CHAPTER_SCENE_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn chapter_scene_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = chapter_scene_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn chapter_content_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(CHAPTER_CONTENT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            CHAPTER_CONTENT_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn chapter_content_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = chapter_content_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn workflow_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(WORKFLOW_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            WORKFLOW_READ_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn workflow_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = workflow_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn config_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(CONFIG_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            CONFIG_READ_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn config_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = config_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn character_mutation_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(CHARACTER_MUTATION_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            CHARACTER_MUTATION_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn character_mutation_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = character_mutation_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn video_record_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(VIDEO_RECORD_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            VIDEO_RECORD_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn video_record_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = video_record_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn manga_record_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(MANGA_RECORD_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            MANGA_RECORD_WORKER_CATALOG_NAME
        )
    })
}

pub(crate) fn manga_record_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = manga_record_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

pub(crate) fn ensure_nonzero_port(name: &str, address: SocketAddr) -> anyhow::Result<SocketAddr> {
    if address.port() == 0 {
        anyhow::bail!("{name} 不允许使用端口 0；Edge 必须知道 Control 的固定地址");
    }
    Ok(address)
}

pub(crate) fn control_binary() -> anyhow::Result<PathBuf> {
    child_binary(CONTROL_BINARY_ENV, "a4n-control")
}

pub(crate) fn edge_binary() -> anyhow::Result<PathBuf> {
    child_binary(EDGE_BINARY_ENV, "a4n-edge")
}

pub(crate) fn agent_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(AGENT_WORKER_BINARY_ENV, &agent_worker_metadata()?.binary)
}

pub(crate) fn novel_planning_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        NOVEL_PLANNING_WORKER_BINARY_ENV,
        &novel_planning_worker_metadata()?.binary,
    )
}

pub(crate) fn artifact_media_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        ARTIFACT_MEDIA_WORKER_BINARY_ENV,
        &artifact_media_worker_metadata()?.binary,
    )
}

pub(crate) fn outline_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        OUTLINE_WORKER_BINARY_ENV,
        &outline_worker_metadata()?.binary,
    )
}

pub(crate) fn blueprint_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        BLUEPRINT_WORKER_BINARY_ENV,
        &blueprint_worker_metadata()?.binary,
    )
}

pub(crate) fn asset_graph_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        ASSET_GRAPH_WORKER_BINARY_ENV,
        &asset_graph_worker_metadata()?.binary,
    )
}

pub(crate) fn foreshadowing_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        FORESHADOWING_WORKER_BINARY_ENV,
        &foreshadowing_worker_metadata()?.binary,
    )
}

pub(crate) fn object_profile_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        OBJECT_PROFILE_WORKER_BINARY_ENV,
        &object_profile_worker_metadata()?.binary,
    )
}

pub(crate) fn scene_profile_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        SCENE_PROFILE_WORKER_BINARY_ENV,
        &scene_profile_worker_metadata()?.binary,
    )
}

pub(crate) fn profile_material_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROFILE_MATERIAL_WORKER_BINARY_ENV,
        &profile_material_worker_metadata()?.binary,
    )
}

pub(crate) fn project_status_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_STATUS_WORKER_BINARY_ENV,
        &project_status_worker_metadata()?.binary,
    )
}

pub(crate) fn portrait_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PORTRAIT_WORKER_BINARY_ENV,
        &portrait_worker_metadata()?.binary,
    )
}

pub(crate) fn source_snapshot_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        SOURCE_SNAPSHOT_WORKER_BINARY_ENV,
        &source_snapshot_worker_metadata()?.binary,
    )
}

pub(crate) fn theme_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        THEME_READ_WORKER_BINARY_ENV,
        &theme_read_worker_metadata()?.binary,
    )
}

pub(crate) fn project_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_READ_WORKER_BINARY_ENV,
        &project_read_worker_metadata()?.binary,
    )
}

pub(crate) fn import_review_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        IMPORT_REVIEW_READ_WORKER_BINARY_ENV,
        &import_review_read_worker_metadata()?.binary,
    )
}

pub(crate) fn inspiration_history_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        INSPIRATION_HISTORY_WORKER_BINARY_ENV,
        &inspiration_history_worker_metadata()?.binary,
    )
}

pub(crate) fn project_access_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_ACCESS_WORKER_BINARY_ENV,
        &project_access_worker_metadata()?.binary,
    )
}

pub(crate) fn project_collaboration_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_COLLABORATION_WORKER_BINARY_ENV,
        &project_collaboration_worker_metadata()?.binary,
    )
}

pub(crate) fn project_mutation_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_MUTATION_WORKER_BINARY_ENV,
        &project_mutation_worker_metadata()?.binary,
    )
}

pub(crate) fn chapter_scene_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        CHAPTER_SCENE_WORKER_BINARY_ENV,
        &chapter_scene_worker_metadata()?.binary,
    )
}

pub(crate) fn chapter_content_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        CHAPTER_CONTENT_WORKER_BINARY_ENV,
        &chapter_content_worker_metadata()?.binary,
    )
}

pub(crate) fn workflow_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        WORKFLOW_READ_WORKER_BINARY_ENV,
        &workflow_read_worker_metadata()?.binary,
    )
}

pub(crate) fn config_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        CONFIG_READ_WORKER_BINARY_ENV,
        &config_read_worker_metadata()?.binary,
    )
}

pub(crate) fn character_mutation_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        CHARACTER_MUTATION_WORKER_BINARY_ENV,
        &character_mutation_worker_metadata()?.binary,
    )
}

pub(crate) fn video_record_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        VIDEO_RECORD_WORKER_BINARY_ENV,
        &video_record_worker_metadata()?.binary,
    )
}

pub(crate) fn manga_record_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        MANGA_RECORD_WORKER_BINARY_ENV,
        &manga_record_worker_metadata()?.binary,
    )
}

pub(crate) fn child_binary(env_name: &str, name: &str) -> anyhow::Result<PathBuf> {
    if let Some(path) = env::var_os(env_name).map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        anyhow::bail!("{env_name} 指向的文件不存在: {}", path.display());
    }

    if let Some(path) = sibling_binary(name) {
        return Ok(path);
    }

    anyhow::bail!("未找到 {name}；请设置 {env_name} 或将其放在 a4n-server 同目录")
}

pub(crate) fn sibling_binary(name: &str) -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    let parent = executable.parent()?;
    let path = parent.join(executable_name(name));
    path.is_file().then_some(path)
}

#[cfg(windows)]
pub(crate) fn executable_name(name: &str) -> PathBuf {
    format!("{name}.exe").into()
}

#[cfg(not(windows))]
pub(crate) fn executable_name(name: &str) -> PathBuf {
    name.into()
}
