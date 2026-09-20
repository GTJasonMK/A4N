use super::*;

pub(crate) fn supervise_runtime_children(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_addr: SocketAddr,
    agent_worker_addr: SocketAddr,
    worker_token: &str,
    control_binary: &Path,
    control_fingerprint: &mut BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: &mut BinaryFingerprint,
    novel_planning_worker_addr: SocketAddr,
    novel_planning_worker_token: &str,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: &mut BinaryFingerprint,
    artifact_media_worker_addr: SocketAddr,
    artifact_media_worker_token: &str,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: &mut BinaryFingerprint,
    outline_worker_addr: SocketAddr,
    outline_worker_token: &str,
    outline_worker: &mut Child,
    outline_worker_binary: &Path,
    outline_worker_fingerprint: &mut BinaryFingerprint,
    blueprint_worker_addr: SocketAddr,
    blueprint_worker_token: &str,
    blueprint_worker: &mut Child,
    blueprint_worker_binary: &Path,
    blueprint_worker_fingerprint: &mut BinaryFingerprint,
    asset_graph_worker_addr: SocketAddr,
    asset_graph_worker_token: &str,
    asset_graph_worker: &mut Child,
    asset_graph_worker_binary: &Path,
    asset_graph_worker_fingerprint: &mut BinaryFingerprint,
    foreshadowing_worker_addr: SocketAddr,
    foreshadowing_worker_token: &str,
    foreshadowing_worker: &mut Child,
    foreshadowing_worker_binary: &Path,
    foreshadowing_worker_fingerprint: &mut BinaryFingerprint,
    object_profile_worker_addr: SocketAddr,
    object_profile_worker_token: &str,
    object_profile_worker: &mut Child,
    object_profile_worker_binary: &Path,
    object_profile_worker_fingerprint: &mut BinaryFingerprint,
    scene_profile_worker_addr: SocketAddr,
    scene_profile_worker_token: &str,
    scene_profile_worker: &mut Child,
    scene_profile_worker_binary: &Path,
    scene_profile_worker_fingerprint: &mut BinaryFingerprint,
    profile_material_worker_addr: SocketAddr,
    profile_material_worker_token: &str,
    profile_material_worker: &mut Child,
    profile_material_worker_binary: &Path,
    profile_material_worker_fingerprint: &mut BinaryFingerprint,
    project_status_worker_addr: SocketAddr,
    project_status_worker_token: &str,
    project_status_worker: &mut Child,
    project_status_worker_binary: &Path,
    project_status_worker_fingerprint: &mut BinaryFingerprint,
    portrait_worker_addr: SocketAddr,
    portrait_worker_token: &str,
    portrait_worker: &mut Child,
    portrait_worker_binary: &Path,
    portrait_worker_fingerprint: &mut BinaryFingerprint,
    source_snapshot_worker_addr: SocketAddr,
    source_snapshot_worker_token: &str,
    source_snapshot_worker: &mut Child,
    source_snapshot_worker_binary: &Path,
    source_snapshot_worker_fingerprint: &mut BinaryFingerprint,
    theme_read_worker_addr: SocketAddr,
    theme_read_worker_token: &str,
    theme_read_worker: &mut Child,
    theme_read_worker_binary: &Path,
    theme_read_worker_fingerprint: &mut BinaryFingerprint,
    project_read_worker_addr: SocketAddr,
    project_read_worker_token: &str,
    project_read_worker: &mut Child,
    project_read_worker_binary: &Path,
    project_read_worker_fingerprint: &mut BinaryFingerprint,
    import_review_read_worker_addr: SocketAddr,
    import_review_read_worker_token: &str,
    import_review_read_worker: &mut Child,
    import_review_read_worker_binary: &Path,
    import_review_read_worker_fingerprint: &mut BinaryFingerprint,
    inspiration_history_worker_addr: SocketAddr,
    inspiration_history_worker_token: &str,
    inspiration_history_worker: &mut Child,
    inspiration_history_worker_binary: &Path,
    inspiration_history_worker_fingerprint: &mut BinaryFingerprint,
    project_access_worker_addr: SocketAddr,
    project_access_worker_token: &str,
    project_access_worker: &mut Child,
    project_access_worker_binary: &Path,
    project_access_worker_fingerprint: &mut BinaryFingerprint,
    project_collaboration_worker_addr: SocketAddr,
    project_collaboration_worker_token: &str,
    project_collaboration_worker: &mut Child,
    project_collaboration_worker_binary: &Path,
    project_collaboration_worker_fingerprint: &mut BinaryFingerprint,
    project_mutation_worker_addr: SocketAddr,
    project_mutation_worker_token: &str,
    project_mutation_worker: &mut Child,
    project_mutation_worker_binary: &Path,
    project_mutation_worker_fingerprint: &mut BinaryFingerprint,
    chapter_scene_worker_addr: SocketAddr,
    chapter_scene_worker_token: &str,
    chapter_scene_worker: &mut Child,
    chapter_scene_worker_binary: &Path,
    chapter_scene_worker_fingerprint: &mut BinaryFingerprint,
    chapter_content_worker_addr: SocketAddr,
    chapter_content_worker_token: &str,
    chapter_content_worker: &mut Child,
    chapter_content_worker_binary: &Path,
    chapter_content_worker_fingerprint: &mut BinaryFingerprint,
    workflow_read_worker_addr: SocketAddr,
    workflow_read_worker_token: &str,
    workflow_read_worker: &mut Child,
    workflow_read_worker_binary: &Path,
    workflow_read_worker_fingerprint: &mut BinaryFingerprint,
    config_read_worker_addr: SocketAddr,
    config_read_worker_token: &str,
    config_read_worker: &mut Child,
    config_read_worker_binary: &Path,
    config_read_worker_fingerprint: &mut BinaryFingerprint,
    character_mutation_worker_addr: SocketAddr,
    character_mutation_worker_token: &str,
    character_mutation_worker: &mut Child,
    character_mutation_worker_binary: &Path,
    character_mutation_worker_fingerprint: &mut BinaryFingerprint,
    video_record_worker_addr: SocketAddr,
    video_record_worker_token: &str,
    video_record_worker: &mut Child,
    video_record_worker_binary: &Path,
    video_record_worker_fingerprint: &mut BinaryFingerprint,
    manga_record_worker_addr: SocketAddr,
    manga_record_worker_token: &str,
    manga_record_worker: &mut Child,
    manga_record_worker_binary: &Path,
    manga_record_worker_fingerprint: &mut BinaryFingerprint,
    edge_binary: &Path,
    edge_fingerprint: &mut BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: &mut Option<BinaryFingerprint>,
) -> anyhow::Result<Option<String>> {
    loop {
        let Some(event) = wait_for_edge_or_child_with_mutation(
            edge,
            control,
            agent_worker,
            control_binary,
            *control_fingerprint,
            worker_binary,
            *worker_fingerprint,
            novel_planning_worker,
            novel_planning_worker_binary,
            *novel_planning_worker_fingerprint,
            artifact_media_worker,
            artifact_media_worker_binary,
            *artifact_media_worker_fingerprint,
            Some((
                outline_worker,
                outline_worker_binary,
                *outline_worker_fingerprint,
            )),
            Some((
                blueprint_worker,
                blueprint_worker_binary,
                *blueprint_worker_fingerprint,
            )),
            Some((
                asset_graph_worker,
                asset_graph_worker_binary,
                *asset_graph_worker_fingerprint,
            )),
            Some((
                foreshadowing_worker,
                foreshadowing_worker_binary,
                *foreshadowing_worker_fingerprint,
            )),
            Some((
                object_profile_worker,
                object_profile_worker_binary,
                *object_profile_worker_fingerprint,
            )),
            Some((
                scene_profile_worker,
                scene_profile_worker_binary,
                *scene_profile_worker_fingerprint,
            )),
            Some((
                profile_material_worker,
                profile_material_worker_binary,
                *profile_material_worker_fingerprint,
            )),
            Some((
                project_status_worker,
                project_status_worker_binary,
                *project_status_worker_fingerprint,
            )),
            Some((
                portrait_worker,
                portrait_worker_binary,
                *portrait_worker_fingerprint,
            )),
            Some((
                source_snapshot_worker,
                source_snapshot_worker_binary,
                *source_snapshot_worker_fingerprint,
            )),
            Some((
                theme_read_worker,
                theme_read_worker_binary,
                *theme_read_worker_fingerprint,
            )),
            Some((
                project_read_worker,
                project_read_worker_binary,
                *project_read_worker_fingerprint,
            )),
            Some((
                import_review_read_worker,
                import_review_read_worker_binary,
                *import_review_read_worker_fingerprint,
            )),
            Some((
                inspiration_history_worker,
                inspiration_history_worker_binary,
                *inspiration_history_worker_fingerprint,
            )),
            Some((
                project_access_worker,
                project_access_worker_binary,
                *project_access_worker_fingerprint,
            )),
            Some((
                project_collaboration_worker,
                project_collaboration_worker_binary,
                *project_collaboration_worker_fingerprint,
            )),
            Some((
                project_mutation_worker,
                project_mutation_worker_binary,
                *project_mutation_worker_fingerprint,
            )),
            Some((
                chapter_scene_worker,
                chapter_scene_worker_binary,
                *chapter_scene_worker_fingerprint,
            )),
            Some((
                chapter_content_worker,
                chapter_content_worker_binary,
                *chapter_content_worker_fingerprint,
            )),
            Some((
                workflow_read_worker,
                workflow_read_worker_binary,
                *workflow_read_worker_fingerprint,
            )),
            Some((
                config_read_worker,
                config_read_worker_binary,
                *config_read_worker_fingerprint,
            )),
            Some((
                character_mutation_worker,
                character_mutation_worker_binary,
                *character_mutation_worker_fingerprint,
            )),
            Some((
                video_record_worker,
                video_record_worker_binary,
                *video_record_worker_fingerprint,
            )),
            Some((
                manga_record_worker,
                manga_record_worker_binary,
                *manga_record_worker_fingerprint,
            )),
            edge_binary,
            *edge_fingerprint,
            reload_marker,
            *reload_marker_fingerprint,
        )?
        else {
            return Ok(None);
        };

        match event {
            ChildWatchEvent::EdgeChanged => {
                eprintln!("检测到 A4N Edge 构建完成；只替换 Edge");
                stop_edge(edge)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *edge = spawn_edge(edge_binary, control_addr)?;
                *edge_fingerprint = binary_fingerprint(edge_binary)?;
            }
            ChildWatchEvent::ControlChanged => {
                eprintln!("检测到 A4N Control 构建完成；只替换 Control");
                stop_control(control)?;
                *control = spawn_control(
                    control_binary,
                    control_addr,
                    worker_token,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                    outline_worker_addr,
                    outline_worker_token,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                    project_status_worker_addr,
                    project_status_worker_token,
                    portrait_worker_addr,
                    portrait_worker_token,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                    project_read_worker_addr,
                    project_read_worker_token,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                    project_access_worker_addr,
                    project_access_worker_token,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                    chapter_scene_worker_addr,
                    chapter_scene_worker_token,
                    chapter_content_worker_addr,
                    chapter_content_worker_token,
                    workflow_read_worker_addr,
                    workflow_read_worker_token,
                    config_read_worker_addr,
                    config_read_worker_token,
                    character_mutation_worker_addr,
                    character_mutation_worker_token,
                    video_record_worker_addr,
                    video_record_worker_token,
                    manga_record_worker_addr,
                    manga_record_worker_token,
                )?;
                wait_for_control(control, control_addr)?;
                *control_fingerprint = binary_fingerprint(control_binary)?;
            }
            ChildWatchEvent::WorkerChanged => {
                eprintln!("检测到 A4N Agent worker 构建完成；只替换 Agent worker");
                stop_agent_worker(agent_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *agent_worker = spawn_agent_worker(
                    worker_binary,
                    control_addr,
                    agent_worker_addr,
                    worker_token,
                )?;
                wait_for_agent_worker(agent_worker, agent_worker_addr, worker_token)?;
                *worker_fingerprint = binary_fingerprint(worker_binary)?;
            }
            ChildWatchEvent::NovelPlanningWorkerChanged => {
                eprintln!(
                    "检测到 A4N Novel Planning worker 构建完成；只替换 Novel Planning worker"
                );
                stop_novel_planning_worker(novel_planning_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *novel_planning_worker = spawn_novel_planning_worker(
                    novel_planning_worker_binary,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                )?;
                wait_for_novel_planning_worker(
                    novel_planning_worker,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                )?;
                *novel_planning_worker_fingerprint =
                    binary_fingerprint(novel_planning_worker_binary)?;
            }
            ChildWatchEvent::ArtifactMediaWorkerChanged => {
                eprintln!(
                    "检测到 A4N Artifact/Media worker 构建完成；只替换 Artifact/Media worker"
                );
                stop_artifact_media_worker(artifact_media_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *artifact_media_worker = spawn_artifact_media_worker(
                    artifact_media_worker_binary,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                )?;
                wait_for_artifact_media_worker(
                    artifact_media_worker,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                )?;
                *artifact_media_worker_fingerprint =
                    binary_fingerprint(artifact_media_worker_binary)?;
            }
            ChildWatchEvent::OutlineWorkerChanged => {
                eprintln!("检测到 A4N Outline worker 构建完成；只替换 Outline worker");
                stop_outline_worker(outline_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *outline_worker = spawn_outline_worker(
                    outline_worker_binary,
                    outline_worker_addr,
                    outline_worker_token,
                )?;
                wait_for_outline_worker(outline_worker, outline_worker_addr, outline_worker_token)?;
                *outline_worker_fingerprint = binary_fingerprint(outline_worker_binary)?;
            }
            ChildWatchEvent::BlueprintWorkerChanged => {
                eprintln!("检测到 A4N Blueprint worker 构建完成；只替换 Blueprint worker");
                stop_blueprint_worker(blueprint_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *blueprint_worker = spawn_blueprint_worker(
                    blueprint_worker_binary,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                )?;
                wait_for_blueprint_worker(
                    blueprint_worker,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                )?;
                *blueprint_worker_fingerprint = binary_fingerprint(blueprint_worker_binary)?;
            }
            ChildWatchEvent::AssetGraphWorkerChanged => {
                eprintln!("检测到 A4N Asset Graph worker 构建完成；只替换 Asset Graph worker");
                stop_asset_graph_worker(asset_graph_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *asset_graph_worker = spawn_asset_graph_worker(
                    asset_graph_worker_binary,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                )?;
                wait_for_asset_graph_worker(
                    asset_graph_worker,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                )?;
                *asset_graph_worker_fingerprint = binary_fingerprint(asset_graph_worker_binary)?;
            }
            ChildWatchEvent::ForeshadowingWorkerChanged => {
                eprintln!("检测到 A4N Foreshadowing worker 构建完成；只替换 Foreshadowing worker");
                stop_foreshadowing_worker(foreshadowing_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *foreshadowing_worker = spawn_foreshadowing_worker(
                    foreshadowing_worker_binary,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                )?;
                wait_for_foreshadowing_worker(
                    foreshadowing_worker,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                )?;
                *foreshadowing_worker_fingerprint =
                    binary_fingerprint(foreshadowing_worker_binary)?;
            }
            ChildWatchEvent::ObjectProfileWorkerChanged => {
                eprintln!(
                    "检测到 A4N Object Profile worker 构建完成；只替换 Object Profile worker"
                );
                stop_object_profile_worker(object_profile_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *object_profile_worker = spawn_object_profile_worker(
                    object_profile_worker_binary,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                )?;
                wait_for_object_profile_worker(
                    object_profile_worker,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                )?;
                *object_profile_worker_fingerprint =
                    binary_fingerprint(object_profile_worker_binary)?;
            }
            ChildWatchEvent::SceneProfileWorkerChanged => {
                eprintln!("检测到 A4N Scene Profile worker 构建完成；只替换 Scene Profile worker");
                stop_scene_profile_worker(scene_profile_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *scene_profile_worker = spawn_scene_profile_worker(
                    scene_profile_worker_binary,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                )?;
                wait_for_scene_profile_worker(
                    scene_profile_worker,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                )?;
                *scene_profile_worker_fingerprint =
                    binary_fingerprint(scene_profile_worker_binary)?;
            }
            ChildWatchEvent::ProfileMaterialWorkerChanged => {
                eprintln!(
                    "检测到 A4N Profile Material worker 构建完成；只替换 Profile Material worker"
                );
                stop_profile_material_worker(profile_material_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *profile_material_worker = spawn_profile_material_worker(
                    profile_material_worker_binary,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                )?;
                wait_for_profile_material_worker(
                    profile_material_worker,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                )?;
                *profile_material_worker_fingerprint =
                    binary_fingerprint(profile_material_worker_binary)?;
            }
            ChildWatchEvent::ProjectStatusWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Status worker 构建完成；只替换 Project Status worker"
                );
                stop_project_status_worker(project_status_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_status_worker = spawn_project_status_worker(
                    project_status_worker_binary,
                    project_status_worker_addr,
                    project_status_worker_token,
                )?;
                wait_for_project_status_worker(
                    project_status_worker,
                    project_status_worker_addr,
                    project_status_worker_token,
                )?;
                *project_status_worker_fingerprint =
                    binary_fingerprint(project_status_worker_binary)?;
            }
            ChildWatchEvent::PortraitWorkerChanged => {
                eprintln!("检测到 A4N Portrait worker 构建完成；只替换 Portrait worker");
                stop_portrait_worker(portrait_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *portrait_worker = spawn_portrait_worker(
                    portrait_worker_binary,
                    portrait_worker_addr,
                    portrait_worker_token,
                )?;
                wait_for_portrait_worker(
                    portrait_worker,
                    portrait_worker_addr,
                    portrait_worker_token,
                )?;
                *portrait_worker_fingerprint = binary_fingerprint(portrait_worker_binary)?;
            }
            ChildWatchEvent::SourceSnapshotWorkerChanged => {
                eprintln!(
                    "检测到 A4N Source Snapshot worker 构建完成；只替换 Source Snapshot worker"
                );
                stop_source_snapshot_worker(source_snapshot_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *source_snapshot_worker = spawn_source_snapshot_worker(
                    source_snapshot_worker_binary,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                )?;
                wait_for_source_snapshot_worker(
                    source_snapshot_worker,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                )?;
                *source_snapshot_worker_fingerprint =
                    binary_fingerprint(source_snapshot_worker_binary)?;
            }
            ChildWatchEvent::ThemeReadWorkerChanged => {
                eprintln!("检测到 A4N Theme Read worker 构建完成；只替换 Theme Read worker");
                stop_theme_read_worker(theme_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *theme_read_worker = spawn_theme_read_worker(
                    theme_read_worker_binary,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                )?;
                wait_for_theme_read_worker(
                    theme_read_worker,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                )?;
                *theme_read_worker_fingerprint = binary_fingerprint(theme_read_worker_binary)?;
            }
            ChildWatchEvent::ProjectReadWorkerChanged => {
                eprintln!("检测到 A4N Project Read worker 构建完成；只替换 Project Read worker");
                stop_project_read_worker(project_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_read_worker = spawn_project_read_worker(
                    project_read_worker_binary,
                    project_read_worker_addr,
                    project_read_worker_token,
                )?;
                wait_for_project_read_worker(
                    project_read_worker,
                    project_read_worker_addr,
                    project_read_worker_token,
                )?;
                *project_read_worker_fingerprint = binary_fingerprint(project_read_worker_binary)?;
            }
            ChildWatchEvent::ImportReviewReadWorkerChanged => {
                eprintln!(
                    "检测到 A4N Import Review Read worker 构建完成；只替换 Import Review Read worker"
                );
                stop_import_review_read_worker(import_review_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *import_review_read_worker = spawn_import_review_read_worker(
                    import_review_read_worker_binary,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                )?;
                wait_for_import_review_read_worker(
                    import_review_read_worker,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                )?;
                *import_review_read_worker_fingerprint =
                    binary_fingerprint(import_review_read_worker_binary)?;
            }
            ChildWatchEvent::InspirationHistoryWorkerChanged => {
                eprintln!(
                    "检测到 A4N Inspiration History worker 构建完成；只替换 Inspiration History worker"
                );
                stop_inspiration_history_worker(inspiration_history_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *inspiration_history_worker = spawn_inspiration_history_worker(
                    inspiration_history_worker_binary,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                )?;
                wait_for_inspiration_history_worker(
                    inspiration_history_worker,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                )?;
                *inspiration_history_worker_fingerprint =
                    binary_fingerprint(inspiration_history_worker_binary)?;
            }
            ChildWatchEvent::ProjectAccessWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Access worker 构建完成；只替换 Project Access worker"
                );
                stop_project_access_worker(project_access_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_access_worker = spawn_project_access_worker(
                    project_access_worker_binary,
                    project_access_worker_addr,
                    project_access_worker_token,
                )?;
                wait_for_project_access_worker(
                    project_access_worker,
                    project_access_worker_addr,
                    project_access_worker_token,
                )?;
                *project_access_worker_fingerprint =
                    binary_fingerprint(project_access_worker_binary)?;
            }
            ChildWatchEvent::ProjectCollaborationWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Collaboration worker 构建完成；只替换 Project Collaboration worker"
                );
                stop_project_collaboration_worker(project_collaboration_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_collaboration_worker = spawn_project_collaboration_worker(
                    project_collaboration_worker_binary,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                )?;
                wait_for_project_collaboration_worker(
                    project_collaboration_worker,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                )?;
                *project_collaboration_worker_fingerprint =
                    binary_fingerprint(project_collaboration_worker_binary)?;
            }
            ChildWatchEvent::ProjectMutationWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Mutation worker 构建完成；只替换 Project Mutation worker"
                );
                stop_project_mutation_worker(project_mutation_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_mutation_worker = spawn_project_mutation_worker(
                    project_mutation_worker_binary,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                wait_for_project_mutation_worker(
                    project_mutation_worker,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                *project_mutation_worker_fingerprint =
                    binary_fingerprint(project_mutation_worker_binary)?;
            }
            ChildWatchEvent::ChapterSceneWorkerChanged => {
                eprintln!("检测到 A4N Chapter Scene worker 构建完成；只替换 Chapter Scene worker");
                stop_chapter_scene_worker(chapter_scene_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *chapter_scene_worker = spawn_chapter_scene_worker(
                    chapter_scene_worker_binary,
                    chapter_scene_worker_addr,
                    chapter_scene_worker_token,
                )?;
                wait_for_chapter_scene_worker(
                    chapter_scene_worker,
                    chapter_scene_worker_addr,
                    chapter_scene_worker_token,
                )?;
                *chapter_scene_worker_fingerprint =
                    binary_fingerprint(chapter_scene_worker_binary)?;
            }
            ChildWatchEvent::ChapterContentWorkerChanged => {
                eprintln!(
                    "检测到 A4N Chapter Content worker 构建完成；只替换 Chapter Content worker"
                );
                stop_chapter_content_worker(chapter_content_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *chapter_content_worker = spawn_chapter_content_worker(
                    chapter_content_worker_binary,
                    chapter_content_worker_addr,
                    chapter_content_worker_token,
                )?;
                wait_for_chapter_content_worker(
                    chapter_content_worker,
                    chapter_content_worker_addr,
                    chapter_content_worker_token,
                )?;
                *chapter_content_worker_fingerprint =
                    binary_fingerprint(chapter_content_worker_binary)?;
            }
            ChildWatchEvent::WorkflowReadWorkerChanged => {
                eprintln!("检测到 A4N Workflow Read worker 构建完成；只替换 Workflow Read worker");
                stop_workflow_read_worker(workflow_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *workflow_read_worker = spawn_workflow_read_worker(
                    workflow_read_worker_binary,
                    workflow_read_worker_addr,
                    workflow_read_worker_token,
                )?;
                wait_for_workflow_read_worker(
                    workflow_read_worker,
                    workflow_read_worker_addr,
                    workflow_read_worker_token,
                )?;
                *workflow_read_worker_fingerprint =
                    binary_fingerprint(workflow_read_worker_binary)?;
            }
            ChildWatchEvent::ConfigReadWorkerChanged => {
                eprintln!("检测到 A4N Config Read worker 构建完成；只替换 Config Read worker");
                stop_config_read_worker(config_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *config_read_worker = spawn_config_read_worker(
                    config_read_worker_binary,
                    config_read_worker_addr,
                    config_read_worker_token,
                )?;
                wait_for_config_read_worker(
                    config_read_worker,
                    config_read_worker_addr,
                    config_read_worker_token,
                )?;
                *config_read_worker_fingerprint = binary_fingerprint(config_read_worker_binary)?;
            }
            ChildWatchEvent::CharacterMutationWorkerChanged => {
                eprintln!(
                    "检测到 A4N Character Mutation worker 构建完成；只替换 Character Mutation worker"
                );
                stop_character_mutation_worker(character_mutation_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *character_mutation_worker = spawn_character_mutation_worker(
                    character_mutation_worker_binary,
                    character_mutation_worker_addr,
                    character_mutation_worker_token,
                )?;
                wait_for_character_mutation_worker(
                    character_mutation_worker,
                    character_mutation_worker_addr,
                    character_mutation_worker_token,
                )?;
                *character_mutation_worker_fingerprint =
                    binary_fingerprint(character_mutation_worker_binary)?;
            }
            ChildWatchEvent::VideoRecordWorkerChanged => {
                eprintln!("检测到 A4N Video Record worker 构建完成；只替换 Video Record worker");
                stop_video_record_worker(video_record_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *video_record_worker = spawn_video_record_worker(
                    video_record_worker_binary,
                    video_record_worker_addr,
                    video_record_worker_token,
                )?;
                wait_for_video_record_worker(
                    video_record_worker,
                    video_record_worker_addr,
                    video_record_worker_token,
                )?;
                *video_record_worker_fingerprint = binary_fingerprint(video_record_worker_binary)?;
            }
            ChildWatchEvent::MangaRecordWorkerChanged => {
                eprintln!("检测到 A4N Manga Record worker 构建完成；只替换 Manga Record worker");
                stop_manga_record_worker(manga_record_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *manga_record_worker = spawn_manga_record_worker(
                    manga_record_worker_binary,
                    manga_record_worker_addr,
                    manga_record_worker_token,
                )?;
                wait_for_manga_record_worker(
                    manga_record_worker,
                    manga_record_worker_addr,
                    manga_record_worker_token,
                )?;
                *manga_record_worker_fingerprint = binary_fingerprint(manga_record_worker_binary)?;
            }
            ChildWatchEvent::ReloadMarkerChanged => {
                // The marker is a publication hint from the development watcher.
                // The binary fingerprints above decide which process actually
                // needs replacement; a marker-only change must not restart either.
                *reload_marker_fingerprint = optional_binary_fingerprint(reload_marker)?;
            }

            event => {
                if handle_exited_event(
                    event,
                    edge,
                    control,
                    agent_worker,
                    control_addr,
                    agent_worker_addr,
                    worker_token,
                    control_binary,
                    control_fingerprint,
                    worker_binary,
                    worker_fingerprint,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                    novel_planning_worker,
                    novel_planning_worker_binary,
                    novel_planning_worker_fingerprint,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                    artifact_media_worker,
                    artifact_media_worker_binary,
                    artifact_media_worker_fingerprint,
                    outline_worker_addr,
                    outline_worker_token,
                    outline_worker,
                    outline_worker_binary,
                    outline_worker_fingerprint,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                    blueprint_worker,
                    blueprint_worker_binary,
                    blueprint_worker_fingerprint,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                    asset_graph_worker,
                    asset_graph_worker_binary,
                    asset_graph_worker_fingerprint,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                    foreshadowing_worker,
                    foreshadowing_worker_binary,
                    foreshadowing_worker_fingerprint,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                    object_profile_worker,
                    object_profile_worker_binary,
                    object_profile_worker_fingerprint,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                    scene_profile_worker,
                    scene_profile_worker_binary,
                    scene_profile_worker_fingerprint,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                    profile_material_worker,
                    profile_material_worker_binary,
                    profile_material_worker_fingerprint,
                    project_status_worker_addr,
                    project_status_worker_token,
                    project_status_worker,
                    project_status_worker_binary,
                    project_status_worker_fingerprint,
                    portrait_worker_addr,
                    portrait_worker_token,
                    portrait_worker,
                    portrait_worker_binary,
                    portrait_worker_fingerprint,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                    source_snapshot_worker,
                    source_snapshot_worker_binary,
                    source_snapshot_worker_fingerprint,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                    theme_read_worker,
                    theme_read_worker_binary,
                    theme_read_worker_fingerprint,
                    project_read_worker_addr,
                    project_read_worker_token,
                    project_read_worker,
                    project_read_worker_binary,
                    project_read_worker_fingerprint,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                    import_review_read_worker,
                    import_review_read_worker_binary,
                    import_review_read_worker_fingerprint,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                    inspiration_history_worker,
                    inspiration_history_worker_binary,
                    inspiration_history_worker_fingerprint,
                    project_access_worker_addr,
                    project_access_worker_token,
                    project_access_worker,
                    project_access_worker_binary,
                    project_access_worker_fingerprint,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                    project_collaboration_worker,
                    project_collaboration_worker_binary,
                    project_collaboration_worker_fingerprint,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                    project_mutation_worker,
                    project_mutation_worker_binary,
                    project_mutation_worker_fingerprint,
                    chapter_scene_worker_addr,
                    chapter_scene_worker_token,
                    chapter_scene_worker,
                    chapter_scene_worker_binary,
                    chapter_scene_worker_fingerprint,
                    chapter_content_worker_addr,
                    chapter_content_worker_token,
                    chapter_content_worker,
                    chapter_content_worker_binary,
                    chapter_content_worker_fingerprint,
                    workflow_read_worker_addr,
                    workflow_read_worker_token,
                    workflow_read_worker,
                    workflow_read_worker_binary,
                    workflow_read_worker_fingerprint,
                    config_read_worker_addr,
                    config_read_worker_token,
                    config_read_worker,
                    config_read_worker_binary,
                    config_read_worker_fingerprint,
                    character_mutation_worker_addr,
                    character_mutation_worker_token,
                    character_mutation_worker,
                    character_mutation_worker_binary,
                    character_mutation_worker_fingerprint,
                    video_record_worker_addr,
                    video_record_worker_token,
                    video_record_worker,
                    video_record_worker_binary,
                    video_record_worker_fingerprint,
                    manga_record_worker_addr,
                    manga_record_worker_token,
                    manga_record_worker,
                    manga_record_worker_binary,
                    manga_record_worker_fingerprint,
                    edge_binary,
                    edge_fingerprint,
                )? {
                    continue;
                }
            }
        }
    }
}
