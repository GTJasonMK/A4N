use super::*;

pub(crate) fn spawn_control(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
    novel_planning_worker_addr: SocketAddr,
    novel_planning_worker_token: &str,
    artifact_media_worker_addr: SocketAddr,
    artifact_media_worker_token: &str,
    outline_worker_addr: SocketAddr,
    outline_worker_token: &str,
    blueprint_worker_addr: SocketAddr,
    blueprint_worker_token: &str,
    asset_graph_worker_addr: SocketAddr,
    asset_graph_worker_token: &str,
    foreshadowing_worker_addr: SocketAddr,
    foreshadowing_worker_token: &str,
    object_profile_worker_addr: SocketAddr,
    object_profile_worker_token: &str,
    scene_profile_worker_addr: SocketAddr,
    scene_profile_worker_token: &str,
    profile_material_worker_addr: SocketAddr,
    profile_material_worker_token: &str,
    project_status_worker_addr: SocketAddr,
    project_status_worker_token: &str,
    portrait_worker_addr: SocketAddr,
    portrait_worker_token: &str,
    source_snapshot_worker_addr: SocketAddr,
    source_snapshot_worker_token: &str,
    theme_read_worker_addr: SocketAddr,
    theme_read_worker_token: &str,
    project_read_worker_addr: SocketAddr,
    project_read_worker_token: &str,
    import_review_read_worker_addr: SocketAddr,
    import_review_read_worker_token: &str,
    inspiration_history_worker_addr: SocketAddr,
    inspiration_history_worker_token: &str,
    project_access_worker_addr: SocketAddr,
    project_access_worker_token: &str,
    project_collaboration_worker_addr: SocketAddr,
    project_collaboration_worker_token: &str,
    project_mutation_worker_addr: SocketAddr,
    project_mutation_worker_token: &str,
    chapter_scene_worker_addr: SocketAddr,
    chapter_scene_worker_token: &str,
    chapter_content_worker_addr: SocketAddr,
    chapter_content_worker_token: &str,
    workflow_read_worker_addr: SocketAddr,
    workflow_read_worker_token: &str,
    config_read_worker_addr: SocketAddr,
    config_read_worker_token: &str,
    character_mutation_worker_addr: SocketAddr,
    character_mutation_worker_token: &str,
    video_record_worker_addr: SocketAddr,
    video_record_worker_token: &str,
    manga_record_worker_addr: SocketAddr,
    manga_record_worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    command
        .env("A4N_CONTROL_ADDR", address.to_string())
        .env(AGENT_WORKER_TOKEN_ENV, worker_token)
        .env(
            NOVEL_PLANNING_WORKER_URL_ENV,
            format!("http://{novel_planning_worker_addr}"),
        )
        .env(NOVEL_PLANNING_WORKER_TOKEN_ENV, novel_planning_worker_token)
        .env(
            ARTIFACT_MEDIA_WORKER_URL_ENV,
            format!("http://{artifact_media_worker_addr}"),
        )
        .env(ARTIFACT_MEDIA_WORKER_TOKEN_ENV, artifact_media_worker_token)
        .env(
            OUTLINE_WORKER_URL_ENV,
            format!("http://{outline_worker_addr}"),
        )
        .env(OUTLINE_WORKER_TOKEN_ENV, outline_worker_token)
        .env(
            BLUEPRINT_WORKER_URL_ENV,
            format!("http://{blueprint_worker_addr}"),
        )
        .env(BLUEPRINT_WORKER_TOKEN_ENV, blueprint_worker_token)
        .env(
            ASSET_GRAPH_WORKER_URL_ENV,
            format!("http://{asset_graph_worker_addr}"),
        )
        .env(ASSET_GRAPH_WORKER_TOKEN_ENV, asset_graph_worker_token)
        .env(
            FORESHADOWING_WORKER_URL_ENV,
            format!("http://{foreshadowing_worker_addr}"),
        )
        .env(FORESHADOWING_WORKER_TOKEN_ENV, foreshadowing_worker_token)
        .env(
            OBJECT_PROFILE_WORKER_URL_ENV,
            format!("http://{object_profile_worker_addr}"),
        )
        .env(OBJECT_PROFILE_WORKER_TOKEN_ENV, object_profile_worker_token)
        .env(
            SCENE_PROFILE_WORKER_URL_ENV,
            format!("http://{scene_profile_worker_addr}"),
        )
        .env(SCENE_PROFILE_WORKER_TOKEN_ENV, scene_profile_worker_token)
        .env(
            PROFILE_MATERIAL_WORKER_URL_ENV,
            format!("http://{profile_material_worker_addr}"),
        )
        .env(
            PROFILE_MATERIAL_WORKER_TOKEN_ENV,
            profile_material_worker_token,
        )
        .env(
            PROJECT_STATUS_WORKER_URL_ENV,
            format!("http://{project_status_worker_addr}"),
        )
        .env(PROJECT_STATUS_WORKER_TOKEN_ENV, project_status_worker_token)
        .env(
            PORTRAIT_WORKER_URL_ENV,
            format!("http://{portrait_worker_addr}"),
        )
        .env(PORTRAIT_WORKER_TOKEN_ENV, portrait_worker_token)
        .env(
            SOURCE_SNAPSHOT_WORKER_URL_ENV,
            format!("http://{source_snapshot_worker_addr}"),
        )
        .env(
            SOURCE_SNAPSHOT_WORKER_TOKEN_ENV,
            source_snapshot_worker_token,
        )
        .env(
            THEME_READ_WORKER_URL_ENV,
            format!("http://{theme_read_worker_addr}"),
        )
        .env(THEME_READ_WORKER_TOKEN_ENV, theme_read_worker_token)
        .env(
            PROJECT_READ_WORKER_URL_ENV,
            format!("http://{project_read_worker_addr}"),
        )
        .env(PROJECT_READ_WORKER_TOKEN_ENV, project_read_worker_token)
        .env(
            IMPORT_REVIEW_READ_WORKER_URL_ENV,
            format!("http://{import_review_read_worker_addr}"),
        )
        .env(
            IMPORT_REVIEW_READ_WORKER_TOKEN_ENV,
            import_review_read_worker_token,
        )
        .env(
            INSPIRATION_HISTORY_WORKER_URL_ENV,
            format!("http://{inspiration_history_worker_addr}"),
        )
        .env(
            INSPIRATION_HISTORY_WORKER_TOKEN_ENV,
            inspiration_history_worker_token,
        )
        .env(
            PROJECT_ACCESS_WORKER_URL_ENV,
            format!("http://{project_access_worker_addr}"),
        )
        .env(PROJECT_ACCESS_WORKER_TOKEN_ENV, project_access_worker_token)
        .env(
            PROJECT_COLLABORATION_WORKER_URL_ENV,
            format!("http://{project_collaboration_worker_addr}"),
        )
        .env(
            PROJECT_COLLABORATION_WORKER_TOKEN_ENV,
            project_collaboration_worker_token,
        )
        .env(
            PROJECT_MUTATION_WORKER_URL_ENV,
            format!("http://{project_mutation_worker_addr}"),
        )
        .env(
            PROJECT_MUTATION_WORKER_TOKEN_ENV,
            project_mutation_worker_token,
        )
        .env(
            CHAPTER_SCENE_WORKER_URL_ENV,
            format!("http://{chapter_scene_worker_addr}"),
        )
        .env(CHAPTER_SCENE_WORKER_TOKEN_ENV, chapter_scene_worker_token)
        .env(
            CHAPTER_CONTENT_WORKER_URL_ENV,
            format!("http://{chapter_content_worker_addr}"),
        )
        .env(
            CHAPTER_CONTENT_WORKER_TOKEN_ENV,
            chapter_content_worker_token,
        )
        .env(
            WORKFLOW_READ_WORKER_URL_ENV,
            format!("http://{workflow_read_worker_addr}"),
        )
        .env(WORKFLOW_READ_WORKER_TOKEN_ENV, workflow_read_worker_token)
        .env(
            CONFIG_READ_WORKER_URL_ENV,
            format!("http://{config_read_worker_addr}"),
        )
        .env(CONFIG_READ_WORKER_TOKEN_ENV, config_read_worker_token)
        .env(
            CHARACTER_MUTATION_WORKER_URL_ENV,
            format!("http://{character_mutation_worker_addr}"),
        )
        .env(
            CHARACTER_MUTATION_WORKER_TOKEN_ENV,
            character_mutation_worker_token,
        )
        .env(
            VIDEO_RECORD_WORKER_URL_ENV,
            format!("http://{video_record_worker_addr}"),
        )
        .env(VIDEO_RECORD_WORKER_TOKEN_ENV, video_record_worker_token)
        .env(
            MANGA_RECORD_WORKER_URL_ENV,
            format!("http://{manga_record_worker_addr}"),
        )
        .env(MANGA_RECORD_WORKER_TOKEN_ENV, manga_record_worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(CONTROL_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Control 失败: {}: {error}", binary.display()))
}

pub(crate) fn spawn_novel_planning_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = novel_planning_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(NOVEL_PLANNING_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Novel Planning worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_artifact_media_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = artifact_media_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(ARTIFACT_MEDIA_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(ARTIFACT_MEDIA_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Artifact/Media worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_outline_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = outline_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(OUTLINE_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(OUTLINE_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Outline worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_blueprint_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = blueprint_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(BLUEPRINT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(BLUEPRINT_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Blueprint worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_asset_graph_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = asset_graph_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(ASSET_GRAPH_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(ASSET_GRAPH_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Asset Graph worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_foreshadowing_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = foreshadowing_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(FORESHADOWING_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(FORESHADOWING_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Foreshadowing worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_object_profile_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = object_profile_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(OBJECT_PROFILE_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(OBJECT_PROFILE_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Object Profile worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_scene_profile_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = scene_profile_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(SCENE_PROFILE_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(SCENE_PROFILE_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Scene Profile worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_profile_material_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = profile_material_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROFILE_MATERIAL_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROFILE_MATERIAL_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Profile Material worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_project_status_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_status_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_STATUS_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_STATUS_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Status worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_theme_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = theme_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(THEME_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(THEME_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Theme Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_project_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_import_review_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = import_review_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(IMPORT_REVIEW_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(IMPORT_REVIEW_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Import Review Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_inspiration_history_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = inspiration_history_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(INSPIRATION_HISTORY_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(INSPIRATION_HISTORY_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Inspiration History worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_project_access_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_access_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_ACCESS_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_ACCESS_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Access worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_project_collaboration_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_collaboration_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_COLLABORATION_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_COLLABORATION_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Collaboration worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_project_mutation_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_mutation_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_MUTATION_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_MUTATION_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Mutation worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_chapter_scene_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = chapter_scene_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(CHAPTER_SCENE_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(CHAPTER_SCENE_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Chapter Scene worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_chapter_content_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = chapter_content_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(CHAPTER_CONTENT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(CHAPTER_CONTENT_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Chapter Content worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_workflow_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = workflow_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(WORKFLOW_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(WORKFLOW_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Workflow Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_config_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = config_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(CONFIG_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(CONFIG_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Config Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_character_mutation_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = character_mutation_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(CHARACTER_MUTATION_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(CHARACTER_MUTATION_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Character Mutation worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_video_record_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = video_record_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(VIDEO_RECORD_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(VIDEO_RECORD_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Video Record worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_manga_record_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = manga_record_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(MANGA_RECORD_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(MANGA_RECORD_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Manga Record worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_portrait_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = portrait_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PORTRAIT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PORTRAIT_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Portrait worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_source_snapshot_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = source_snapshot_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(SOURCE_SNAPSHOT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(SOURCE_SNAPSHOT_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Source Snapshot worker 失败: {}: {error}",
            binary.display()
        )
    })
}

pub(crate) fn spawn_edge(binary: &Path, control_addr: SocketAddr) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    command
        .env("A4N_CONTROL_ADDR", control_addr.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(EDGE_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Edge 失败: {}: {error}", binary.display()))
}

pub(crate) fn spawn_agent_worker(
    binary: &Path,
    address: SocketAddr,
    worker_address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let worker_addr_env = agent_worker_metadata()?.addr_env.clone();
    command
        .env("A4N_CONTROL_ADDR", address.to_string())
        .env(worker_addr_env, worker_address.to_string())
        .env(AGENT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env("A4N_AGENT_WORKER_PROCESS", "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!("启动 A4N Agent worker 失败: {}: {error}", binary.display())
    })
}

pub(crate) fn stop_agent_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_novel_planning_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_artifact_media_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_outline_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_blueprint_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_asset_graph_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_foreshadowing_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_object_profile_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_scene_profile_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_profile_material_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_project_status_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_theme_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_project_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_import_review_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_inspiration_history_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_project_access_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_project_collaboration_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_project_mutation_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_chapter_scene_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_chapter_content_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_workflow_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_config_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_character_mutation_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_video_record_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_manga_record_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_portrait_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_source_snapshot_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_control(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_edge(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_child(child: &mut Child) -> anyhow::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
        for _ in 0..50 {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    child.kill()?;
    child.wait()?;
    Ok(())
}

pub(crate) fn finish_shutdown(
    edge_result: anyhow::Result<()>,
    worker_result: anyhow::Result<()>,
    control_result: anyhow::Result<()>,
    novel_planning_worker_result: anyhow::Result<()>,
    artifact_media_worker_result: anyhow::Result<()>,
    outline_worker_result: anyhow::Result<()>,
    blueprint_worker_result: anyhow::Result<()>,
    asset_graph_worker_result: anyhow::Result<()>,
    foreshadowing_worker_result: anyhow::Result<()>,
    object_profile_worker_result: anyhow::Result<()>,
    scene_profile_worker_result: anyhow::Result<()>,
    profile_material_worker_result: anyhow::Result<()>,
    project_status_worker_result: anyhow::Result<()>,
    portrait_worker_result: anyhow::Result<()>,
    source_snapshot_worker_result: anyhow::Result<()>,
    theme_read_worker_result: anyhow::Result<()>,
    project_read_worker_result: anyhow::Result<()>,
    import_review_read_worker_result: anyhow::Result<()>,
    inspiration_history_worker_result: anyhow::Result<()>,
    project_access_worker_result: anyhow::Result<()>,
    project_collaboration_worker_result: anyhow::Result<()>,
    project_mutation_worker_result: anyhow::Result<()>,
    chapter_scene_worker_result: anyhow::Result<()>,
    chapter_content_worker_result: anyhow::Result<()>,
    workflow_read_worker_result: anyhow::Result<()>,
    config_read_worker_result: anyhow::Result<()>,
    character_mutation_worker_result: anyhow::Result<()>,
    video_record_worker_result: anyhow::Result<()>,
    manga_record_worker_result: anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut failures = Vec::new();
    for (name, result) in [
        ("Edge", edge_result),
        ("Agent worker", worker_result),
        ("Control", control_result),
        ("Novel Planning worker", novel_planning_worker_result),
        ("Artifact/Media worker", artifact_media_worker_result),
        ("Outline worker", outline_worker_result),
        ("Blueprint worker", blueprint_worker_result),
        ("Asset Graph worker", asset_graph_worker_result),
        ("Foreshadowing worker", foreshadowing_worker_result),
        ("Object Profile worker", object_profile_worker_result),
        ("Scene Profile worker", scene_profile_worker_result),
        ("Profile Material worker", profile_material_worker_result),
        ("Project Status worker", project_status_worker_result),
        ("Portrait worker", portrait_worker_result),
        ("Source Snapshot worker", source_snapshot_worker_result),
        ("Theme Read worker", theme_read_worker_result),
        ("Project Read worker", project_read_worker_result),
        (
            "Import Review Read worker",
            import_review_read_worker_result,
        ),
        (
            "Inspiration History worker",
            inspiration_history_worker_result,
        ),
        ("Project Access worker", project_access_worker_result),
        (
            "Project Collaboration worker",
            project_collaboration_worker_result,
        ),
        ("Project Mutation worker", project_mutation_worker_result),
        ("Chapter Scene worker", chapter_scene_worker_result),
        ("Chapter Content worker", chapter_content_worker_result),
        ("Workflow Read worker", workflow_read_worker_result),
        ("Config Read worker", config_read_worker_result),
        (
            "Character Mutation worker",
            character_mutation_worker_result,
        ),
        ("Video Record worker", video_record_worker_result),
        ("Manga Record worker", manga_record_worker_result),
    ] {
        if let Err(error) = result {
            failures.push(format!("{name}: {error:#}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(failures.join("；"));
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn configure_parent_death_signal(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn prctl(option: i32, ...) -> i32;
            }
            const PR_SET_PDEATHSIG: i32 = 1;
            const SIGTERM: i32 = 15;
            if prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn configure_parent_death_signal(_: &mut Command) {}
