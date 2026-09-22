use super::super::*;

#[cfg(test)]
pub(crate) fn wait_for_edge_or_child(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    wait_for_edge_or_child_with_outline(
        edge,
        control,
        agent_worker,
        control_binary,
        control_fingerprint,
        worker_binary,
        worker_fingerprint,
        novel_planning_worker,
        novel_planning_worker_binary,
        novel_planning_worker_fingerprint,
        artifact_media_worker,
        artifact_media_worker_binary,
        artifact_media_worker_fingerprint,
        None,
        None,
        None,
        edge_binary,
        edge_fingerprint,
        reload_marker,
        reload_marker_fingerprint,
    )
}

#[cfg(test)]
pub(crate) fn wait_for_edge_or_child_with_outline(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    outline_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    blueprint_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    asset_graph_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    wait_for_edge_or_child_with_mutation(
        edge,
        control,
        agent_worker,
        control_binary,
        control_fingerprint,
        worker_binary,
        worker_fingerprint,
        novel_planning_worker,
        novel_planning_worker_binary,
        novel_planning_worker_fingerprint,
        artifact_media_worker,
        artifact_media_worker_binary,
        artifact_media_worker_fingerprint,
        outline_worker,
        blueprint_worker,
        asset_graph_worker,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        edge_binary,
        edge_fingerprint,
        reload_marker,
        reload_marker_fingerprint,
    )
}

#[cfg(test)]
pub(crate) fn wait_for_edge_or_child_with_workers(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    outline_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    blueprint_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    asset_graph_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    foreshadowing_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    object_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    scene_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    profile_material_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_status_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    portrait_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    source_snapshot_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    theme_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    import_review_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    inspiration_history_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_access_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_collaboration_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    wait_for_edge_or_child_with_mutation(
        edge,
        control,
        agent_worker,
        control_binary,
        control_fingerprint,
        worker_binary,
        worker_fingerprint,
        novel_planning_worker,
        novel_planning_worker_binary,
        novel_planning_worker_fingerprint,
        artifact_media_worker,
        artifact_media_worker_binary,
        artifact_media_worker_fingerprint,
        outline_worker,
        blueprint_worker,
        asset_graph_worker,
        foreshadowing_worker,
        object_profile_worker,
        scene_profile_worker,
        profile_material_worker,
        project_status_worker,
        portrait_worker,
        source_snapshot_worker,
        theme_read_worker,
        project_read_worker,
        import_review_read_worker,
        inspiration_history_worker,
        project_access_worker,
        project_collaboration_worker,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        edge_binary,
        edge_fingerprint,
        reload_marker,
        reload_marker_fingerprint,
    )
}

pub(crate) fn wait_for_edge_or_child_with_mutation(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    mut outline_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut blueprint_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut asset_graph_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut foreshadowing_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut object_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut scene_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut profile_material_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_status_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut portrait_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut source_snapshot_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut theme_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut import_review_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut inspiration_history_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_access_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_collaboration_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_mutation_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut chapter_scene_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut chapter_content_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut workflow_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut config_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut character_mutation_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut video_record_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut manga_record_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    loop {
        if let Some(status) = edge.try_wait()? {
            return Ok(Some(ChildWatchEvent::EdgeExited(status)));
        }
        if let Some(status) = control.try_wait()? {
            return Ok(Some(ChildWatchEvent::ControlExited(status)));
        }
        if let Some(status) = agent_worker.try_wait()? {
            return Ok(Some(ChildWatchEvent::WorkerExited(status)));
        }
        if let Some(status) = novel_planning_worker.try_wait()? {
            return Ok(Some(ChildWatchEvent::NovelPlanningWorkerExited(status)));
        }
        if let Some(status) = artifact_media_worker.try_wait()? {
            return Ok(Some(ChildWatchEvent::ArtifactMediaWorkerExited(status)));
        }
        if let Some((outline_worker, _, _)) = outline_worker.as_mut()
            && let Some(status) = outline_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::OutlineWorkerExited(status)));
        }
        if let Some((blueprint_worker, _, _)) = blueprint_worker.as_mut()
            && let Some(status) = blueprint_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::BlueprintWorkerExited(status)));
        }
        if let Some((asset_graph_worker, _, _)) = asset_graph_worker.as_mut()
            && let Some(status) = asset_graph_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::AssetGraphWorkerExited(status)));
        }
        if let Some((foreshadowing_worker, _, _)) = foreshadowing_worker.as_mut()
            && let Some(status) = foreshadowing_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ForeshadowingWorkerExited(status)));
        }
        if let Some((object_profile_worker, _, _)) = object_profile_worker.as_mut()
            && let Some(status) = object_profile_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ObjectProfileWorkerExited(status)));
        }
        if let Some((scene_profile_worker, _, _)) = scene_profile_worker.as_mut()
            && let Some(status) = scene_profile_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::SceneProfileWorkerExited(status)));
        }
        if let Some((profile_material_worker, _, _)) = profile_material_worker.as_mut()
            && let Some(status) = profile_material_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProfileMaterialWorkerExited(status)));
        }
        if let Some((project_status_worker, _, _)) = project_status_worker.as_mut()
            && let Some(status) = project_status_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectStatusWorkerExited(status)));
        }
        if let Some((portrait_worker, _, _)) = portrait_worker.as_mut()
            && let Some(status) = portrait_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::PortraitWorkerExited(status)));
        }
        if let Some((source_snapshot_worker, _, _)) = source_snapshot_worker.as_mut()
            && let Some(status) = source_snapshot_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::SourceSnapshotWorkerExited(status)));
        }
        if let Some((theme_read_worker, _, _)) = theme_read_worker.as_mut()
            && let Some(status) = theme_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ThemeReadWorkerExited(status)));
        }
        if let Some((project_read_worker, _, _)) = project_read_worker.as_mut()
            && let Some(status) = project_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectReadWorkerExited(status)));
        }
        if let Some((import_review_read_worker, _, _)) = import_review_read_worker.as_mut()
            && let Some(status) = import_review_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ImportReviewReadWorkerExited(status)));
        }
        if let Some((inspiration_history_worker, _, _)) = inspiration_history_worker.as_mut()
            && let Some(status) = inspiration_history_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::InspirationHistoryWorkerExited(
                status,
            )));
        }
        if let Some((project_access_worker, _, _)) = project_access_worker.as_mut()
            && let Some(status) = project_access_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectAccessWorkerExited(status)));
        }
        if let Some((project_collaboration_worker, _, _)) = project_collaboration_worker.as_mut()
            && let Some(status) = project_collaboration_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectCollaborationWorkerExited(
                status,
            )));
        }
        if let Some((project_mutation_worker, _, _)) = project_mutation_worker.as_mut()
            && let Some(status) = project_mutation_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectMutationWorkerExited(status)));
        }
        if let Some((chapter_scene_worker, _, _)) = chapter_scene_worker.as_mut()
            && let Some(status) = chapter_scene_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ChapterSceneWorkerExited(status)));
        }
        if let Some((chapter_content_worker, _, _)) = chapter_content_worker.as_mut()
            && let Some(status) = chapter_content_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ChapterContentWorkerExited(status)));
        }
        if let Some((workflow_read_worker, _, _)) = workflow_read_worker.as_mut()
            && let Some(status) = workflow_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::WorkflowReadWorkerExited(status)));
        }
        if let Some((config_read_worker, _, _)) = config_read_worker.as_mut()
            && let Some(status) = config_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ConfigReadWorkerExited(status)));
        }
        if let Some((character_mutation_worker, _, _)) = character_mutation_worker.as_mut()
            && let Some(status) = character_mutation_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::CharacterMutationWorkerExited(status)));
        }
        if let Some((video_record_worker, _, _)) = video_record_worker.as_mut()
            && let Some(status) = video_record_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::VideoRecordWorkerExited(status)));
        }
        if let Some((manga_record_worker, _, _)) = manga_record_worker.as_mut()
            && let Some(status) = manga_record_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::MangaRecordWorkerExited(status)));
        }
        if binary_fingerprint(edge_binary)? != edge_fingerprint {
            return Ok(Some(ChildWatchEvent::EdgeChanged));
        }
        let control_changed = binary_fingerprint(control_binary)? != control_fingerprint;
        let worker_changed = binary_fingerprint(worker_binary)? != worker_fingerprint;
        let novel_planning_worker_changed =
            binary_fingerprint(novel_planning_worker_binary)? != novel_planning_worker_fingerprint;
        if control_changed {
            return Ok(Some(ChildWatchEvent::ControlChanged));
        }
        if worker_changed {
            return Ok(Some(ChildWatchEvent::WorkerChanged));
        }
        if novel_planning_worker_changed {
            return Ok(Some(ChildWatchEvent::NovelPlanningWorkerChanged));
        }
        let artifact_media_worker_changed =
            binary_fingerprint(artifact_media_worker_binary)? != artifact_media_worker_fingerprint;
        if artifact_media_worker_changed {
            return Ok(Some(ChildWatchEvent::ArtifactMediaWorkerChanged));
        }
        if let Some((_, outline_worker_binary, outline_worker_fingerprint)) =
            outline_worker.as_ref()
            && binary_fingerprint(outline_worker_binary)? != *outline_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::OutlineWorkerChanged));
        }
        if let Some((_, blueprint_worker_binary, blueprint_worker_fingerprint)) =
            blueprint_worker.as_ref()
            && binary_fingerprint(blueprint_worker_binary)? != *blueprint_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::BlueprintWorkerChanged));
        }
        if let Some((_, asset_graph_worker_binary, asset_graph_worker_fingerprint)) =
            asset_graph_worker.as_ref()
            && binary_fingerprint(asset_graph_worker_binary)? != *asset_graph_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::AssetGraphWorkerChanged));
        }
        if let Some((_, foreshadowing_worker_binary, foreshadowing_worker_fingerprint)) =
            foreshadowing_worker.as_ref()
            && binary_fingerprint(foreshadowing_worker_binary)? != *foreshadowing_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ForeshadowingWorkerChanged));
        }
        if let Some((_, object_profile_worker_binary, object_profile_worker_fingerprint)) =
            object_profile_worker.as_ref()
            && binary_fingerprint(object_profile_worker_binary)?
                != *object_profile_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ObjectProfileWorkerChanged));
        }
        if let Some((_, scene_profile_worker_binary, scene_profile_worker_fingerprint)) =
            scene_profile_worker.as_ref()
            && binary_fingerprint(scene_profile_worker_binary)? != *scene_profile_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::SceneProfileWorkerChanged));
        }
        if let Some((_, profile_material_worker_binary, profile_material_worker_fingerprint)) =
            profile_material_worker.as_ref()
            && binary_fingerprint(profile_material_worker_binary)?
                != *profile_material_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProfileMaterialWorkerChanged));
        }
        if let Some((_, project_status_worker_binary, project_status_worker_fingerprint)) =
            project_status_worker.as_ref()
            && binary_fingerprint(project_status_worker_binary)?
                != *project_status_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectStatusWorkerChanged));
        }
        if let Some((_, portrait_worker_binary, portrait_worker_fingerprint)) =
            portrait_worker.as_ref()
            && binary_fingerprint(portrait_worker_binary)? != *portrait_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::PortraitWorkerChanged));
        }
        if let Some((_, source_snapshot_worker_binary, source_snapshot_worker_fingerprint)) =
            source_snapshot_worker.as_ref()
            && binary_fingerprint(source_snapshot_worker_binary)?
                != *source_snapshot_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::SourceSnapshotWorkerChanged));
        }
        if let Some((_, theme_read_worker_binary, theme_read_worker_fingerprint)) =
            theme_read_worker.as_ref()
            && binary_fingerprint(theme_read_worker_binary)? != *theme_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ThemeReadWorkerChanged));
        }
        if let Some((_, project_read_worker_binary, project_read_worker_fingerprint)) =
            project_read_worker.as_ref()
            && binary_fingerprint(project_read_worker_binary)? != *project_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectReadWorkerChanged));
        }
        if let Some((_, import_review_read_worker_binary, import_review_read_worker_fingerprint)) =
            import_review_read_worker.as_ref()
            && binary_fingerprint(import_review_read_worker_binary)?
                != *import_review_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ImportReviewReadWorkerChanged));
        }
        if let Some((_, inspiration_history_worker_binary, inspiration_history_worker_fingerprint)) =
            inspiration_history_worker.as_ref()
            && binary_fingerprint(inspiration_history_worker_binary)?
                != *inspiration_history_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::InspirationHistoryWorkerChanged));
        }
        if let Some((_, project_access_worker_binary, project_access_worker_fingerprint)) =
            project_access_worker.as_ref()
            && binary_fingerprint(project_access_worker_binary)?
                != *project_access_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectAccessWorkerChanged));
        }
        if let Some((
            _,
            project_collaboration_worker_binary,
            project_collaboration_worker_fingerprint,
        )) = project_collaboration_worker.as_ref()
            && binary_fingerprint(project_collaboration_worker_binary)?
                != *project_collaboration_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectCollaborationWorkerChanged));
        }
        if let Some((_, project_mutation_worker_binary, project_mutation_worker_fingerprint)) =
            project_mutation_worker.as_ref()
            && binary_fingerprint(project_mutation_worker_binary)?
                != *project_mutation_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectMutationWorkerChanged));
        }
        if let Some((_, chapter_scene_worker_binary, chapter_scene_worker_fingerprint)) =
            chapter_scene_worker.as_ref()
            && binary_fingerprint(chapter_scene_worker_binary)? != *chapter_scene_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ChapterSceneWorkerChanged));
        }
        if let Some((_, chapter_content_worker_binary, chapter_content_worker_fingerprint)) =
            chapter_content_worker.as_ref()
            && binary_fingerprint(chapter_content_worker_binary)?
                != *chapter_content_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ChapterContentWorkerChanged));
        }
        if let Some((_, workflow_read_worker_binary, workflow_read_worker_fingerprint)) =
            workflow_read_worker.as_ref()
            && binary_fingerprint(workflow_read_worker_binary)? != *workflow_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::WorkflowReadWorkerChanged));
        }
        if let Some((_, config_read_worker_binary, config_read_worker_fingerprint)) =
            config_read_worker.as_ref()
            && binary_fingerprint(config_read_worker_binary)? != *config_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ConfigReadWorkerChanged));
        }
        if let Some((_, character_mutation_worker_binary, character_mutation_worker_fingerprint)) =
            character_mutation_worker.as_ref()
            && binary_fingerprint(character_mutation_worker_binary)?
                != *character_mutation_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::CharacterMutationWorkerChanged));
        }
        if let Some((_, video_record_worker_binary, video_record_worker_fingerprint)) =
            video_record_worker.as_ref()
            && binary_fingerprint(video_record_worker_binary)? != *video_record_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::VideoRecordWorkerChanged));
        }
        if let Some((_, manga_record_worker_binary, manga_record_worker_fingerprint)) =
            manga_record_worker.as_ref()
            && binary_fingerprint(manga_record_worker_binary)? != *manga_record_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::MangaRecordWorkerChanged));
        }
        if reload_marker.is_some()
            && optional_binary_fingerprint(reload_marker)? != reload_marker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ReloadMarkerChanged));
        }
        std::thread::sleep(CHILD_MONITOR_INTERVAL);
    }
}
