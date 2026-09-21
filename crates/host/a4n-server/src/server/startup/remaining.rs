use super::super::*;
use super::state::{Foundation, Runtime, StartupConfig};

pub(crate) fn start(config: StartupConfig, foundation: Foundation) -> anyhow::Result<Runtime> {
    let reload_marker_fingerprint = config.reload_marker_fingerprint;
    let Foundation {
        mut artifact_media_worker,
        mut novel_planning_worker,
        mut outline_worker,
        mut blueprint_worker,
        mut asset_graph_worker,
        mut foreshadowing_worker,
        mut object_profile_worker,
        mut scene_profile_worker,
        mut portrait_worker,
        mut source_snapshot_worker,
        mut config_read_worker,
        control_fingerprint,
        worker_fingerprint,
        edge_fingerprint,
        novel_planning_worker_fingerprint,
        artifact_media_worker_fingerprint,
        outline_worker_fingerprint,
        blueprint_worker_fingerprint,
        asset_graph_worker_fingerprint,
        foreshadowing_worker_fingerprint,
        object_profile_worker_fingerprint,
        scene_profile_worker_fingerprint,
        profile_material_worker_fingerprint,
        project_status_worker_fingerprint,
        portrait_worker_fingerprint,
        source_snapshot_worker_fingerprint,
        config_read_worker_fingerprint,
    } = foundation;
    let mut control = match spawn_control(
        &config.control_binary,
        config.control_addr,
        &config.worker_token,
        config.novel_planning_worker_addr,
        &config.novel_planning_worker_token,
        config.artifact_media_worker_addr,
        &config.artifact_media_worker_token,
        config.outline_worker_addr,
        &config.outline_worker_token,
        config.blueprint_worker_addr,
        &config.blueprint_worker_token,
        config.asset_graph_worker_addr,
        &config.asset_graph_worker_token,
        config.foreshadowing_worker_addr,
        &config.foreshadowing_worker_token,
        config.object_profile_worker_addr,
        &config.object_profile_worker_token,
        config.scene_profile_worker_addr,
        &config.scene_profile_worker_token,
        config.profile_material_worker_addr,
        &config.profile_material_worker_token,
        config.project_status_worker_addr,
        &config.project_status_worker_token,
        config.portrait_worker_addr,
        &config.portrait_worker_token,
        config.source_snapshot_worker_addr,
        &config.source_snapshot_worker_token,
        config.theme_read_worker_addr,
        &config.theme_read_worker_token,
        config.project_read_worker_addr,
        &config.project_read_worker_token,
        config.import_review_read_worker_addr,
        &config.import_review_read_worker_token,
        config.inspiration_history_worker_addr,
        &config.inspiration_history_worker_token,
        config.project_access_worker_addr,
        &config.project_access_worker_token,
        config.project_collaboration_worker_addr,
        &config.project_collaboration_worker_token,
        config.project_mutation_worker_addr,
        &config.project_mutation_worker_token,
        config.chapter_scene_worker_addr,
        &config.chapter_scene_worker_token,
        config.chapter_content_worker_addr,
        &config.chapter_content_worker_token,
        config.workflow_read_worker_addr,
        &config.workflow_read_worker_token,
        config.config_read_worker_addr,
        &config.config_read_worker_token,
        config.character_mutation_worker_addr,
        &config.character_mutation_worker_token,
        config.video_record_worker_addr,
        &config.video_record_worker_token,
        config.manga_record_worker_addr,
        &config.manga_record_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_control(&mut control, config.control_addr) {
        let cleanup_result = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return match cleanup_result {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(anyhow::anyhow!(
                "A4N Control 未就绪: {error:#}；清理 Control 失败: {cleanup_error:#}"
            )),
        };
    }
    let mut agent_worker = match spawn_agent_worker(
        &config.worker_binary,
        config.control_addr,
        config.agent_worker_addr,
        &config.worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_agent_worker(
        &mut agent_worker,
        config.agent_worker_addr,
        &config.worker_token,
    ) {
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let mut profile_material_worker = match spawn_profile_material_worker(
        &config.profile_material_worker_binary,
        config.profile_material_worker_addr,
        &config.profile_material_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_profile_material_worker(
        &mut profile_material_worker,
        config.profile_material_worker_addr,
        &config.profile_material_worker_token,
    ) {
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let mut project_status_worker = match spawn_project_status_worker(
        &config.project_status_worker_binary,
        config.project_status_worker_addr,
        &config.project_status_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_status_worker(
        &mut project_status_worker,
        config.project_status_worker_addr,
        &config.project_status_worker_token,
    ) {
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let mut theme_read_worker = match spawn_theme_read_worker(
        &config.theme_read_worker_binary,
        config.theme_read_worker_addr,
        &config.theme_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_theme_read_worker(
        &mut theme_read_worker,
        config.theme_read_worker_addr,
        &config.theme_read_worker_token,
    ) {
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let theme_read_worker_fingerprint = binary_fingerprint(&config.theme_read_worker_binary)?;
    let project_read_worker_binary = project_read_worker_binary()?;
    let mut project_read_worker = match spawn_project_read_worker(
        &project_read_worker_binary,
        config.project_read_worker_addr,
        &config.project_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_read_worker(
        &mut project_read_worker,
        config.project_read_worker_addr,
        &config.project_read_worker_token,
    ) {
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let project_read_worker_fingerprint = binary_fingerprint(&project_read_worker_binary)?;
    let mut import_review_read_worker = match spawn_import_review_read_worker(
        &config.import_review_read_worker_binary,
        config.import_review_read_worker_addr,
        &config.import_review_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_import_review_read_worker(
        &mut import_review_read_worker,
        config.import_review_read_worker_addr,
        &config.import_review_read_worker_token,
    ) {
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let import_review_read_worker_fingerprint =
        binary_fingerprint(&config.import_review_read_worker_binary)?;
    let mut inspiration_history_worker = match spawn_inspiration_history_worker(
        &config.inspiration_history_worker_binary,
        config.inspiration_history_worker_addr,
        &config.inspiration_history_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_inspiration_history_worker(
        &mut inspiration_history_worker,
        config.inspiration_history_worker_addr,
        &config.inspiration_history_worker_token,
    ) {
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let inspiration_history_worker_fingerprint =
        binary_fingerprint(&config.inspiration_history_worker_binary)?;
    let mut project_access_worker = match spawn_project_access_worker(
        &config.project_access_worker_binary,
        config.project_access_worker_addr,
        &config.project_access_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_access_worker(
        &mut project_access_worker,
        config.project_access_worker_addr,
        &config.project_access_worker_token,
    ) {
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let project_access_worker_fingerprint =
        binary_fingerprint(&config.project_access_worker_binary)?;
    let mut project_collaboration_worker = match spawn_project_collaboration_worker(
        &config.project_collaboration_worker_binary,
        config.project_collaboration_worker_addr,
        &config.project_collaboration_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_collaboration_worker(
        &mut project_collaboration_worker,
        config.project_collaboration_worker_addr,
        &config.project_collaboration_worker_token,
    ) {
        let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let project_collaboration_worker_fingerprint =
        binary_fingerprint(&config.project_collaboration_worker_binary)?;
    let mut project_mutation_worker = match spawn_project_mutation_worker(
        &config.project_mutation_worker_binary,
        config.project_mutation_worker_addr,
        &config.project_mutation_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_mutation_worker(
        &mut project_mutation_worker,
        config.project_mutation_worker_addr,
        &config.project_mutation_worker_token,
    ) {
        let _ = stop_project_mutation_worker(&mut project_mutation_worker);
        let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let project_mutation_worker_fingerprint =
        binary_fingerprint(&config.project_mutation_worker_binary)?;
    let mut chapter_scene_worker = match spawn_chapter_scene_worker(
        &config.chapter_scene_worker_binary,
        config.chapter_scene_worker_addr,
        &config.chapter_scene_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_mutation_worker(&mut project_mutation_worker);
            let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_chapter_scene_worker(
        &mut chapter_scene_worker,
        config.chapter_scene_worker_addr,
        &config.chapter_scene_worker_token,
    ) {
        let _ = stop_chapter_scene_worker(&mut chapter_scene_worker);
        let _ = stop_project_mutation_worker(&mut project_mutation_worker);
        let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let chapter_scene_worker_fingerprint = binary_fingerprint(&config.chapter_scene_worker_binary)?;
    let mut chapter_content_worker = match spawn_chapter_content_worker(
        &config.chapter_content_worker_binary,
        config.chapter_content_worker_addr,
        &config.chapter_content_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_chapter_scene_worker(&mut chapter_scene_worker);
            let _ = stop_project_mutation_worker(&mut project_mutation_worker);
            let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_chapter_content_worker(
        &mut chapter_content_worker,
        config.chapter_content_worker_addr,
        &config.chapter_content_worker_token,
    ) {
        let _ = stop_chapter_content_worker(&mut chapter_content_worker);
        let _ = stop_chapter_scene_worker(&mut chapter_scene_worker);
        let _ = stop_project_mutation_worker(&mut project_mutation_worker);
        let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let chapter_content_worker_fingerprint =
        binary_fingerprint(&config.chapter_content_worker_binary)?;
    let mut workflow_read_worker = match spawn_workflow_read_worker(
        &config.workflow_read_worker_binary,
        config.workflow_read_worker_addr,
        &config.workflow_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_chapter_content_worker(&mut chapter_content_worker);
            let _ = stop_chapter_scene_worker(&mut chapter_scene_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_workflow_read_worker(
        &mut workflow_read_worker,
        config.workflow_read_worker_addr,
        &config.workflow_read_worker_token,
    ) {
        let _ = stop_workflow_read_worker(&mut workflow_read_worker);
        let _ = stop_chapter_content_worker(&mut chapter_content_worker);
        let _ = stop_chapter_scene_worker(&mut chapter_scene_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        return Err(error);
    }
    let workflow_read_worker_fingerprint = binary_fingerprint(&config.workflow_read_worker_binary)?;
    let mut character_mutation_worker = match spawn_character_mutation_worker(
        &config.character_mutation_worker_binary,
        config.character_mutation_worker_addr,
        &config.character_mutation_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_config_read_worker(&mut config_read_worker);
            let _ = stop_workflow_read_worker(&mut workflow_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_character_mutation_worker(
        &mut character_mutation_worker,
        config.character_mutation_worker_addr,
        &config.character_mutation_worker_token,
    ) {
        let _ = stop_character_mutation_worker(&mut character_mutation_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        let _ = stop_workflow_read_worker(&mut workflow_read_worker);
        return Err(error);
    }
    let character_mutation_worker_fingerprint =
        binary_fingerprint(&config.character_mutation_worker_binary)?;
    let mut video_record_worker = match spawn_video_record_worker(
        &config.video_record_worker_binary,
        config.video_record_worker_addr,
        &config.video_record_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_character_mutation_worker(&mut character_mutation_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            let _ = stop_workflow_read_worker(&mut workflow_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_video_record_worker(
        &mut video_record_worker,
        config.video_record_worker_addr,
        &config.video_record_worker_token,
    ) {
        let _ = stop_video_record_worker(&mut video_record_worker);
        let _ = stop_character_mutation_worker(&mut character_mutation_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        let _ = stop_workflow_read_worker(&mut workflow_read_worker);
        return Err(error);
    }
    let video_record_worker_fingerprint = binary_fingerprint(&config.video_record_worker_binary)?;
    let mut manga_record_worker = match spawn_manga_record_worker(
        &config.manga_record_worker_binary,
        config.manga_record_worker_addr,
        &config.manga_record_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_video_record_worker(&mut video_record_worker);
            let _ = stop_character_mutation_worker(&mut character_mutation_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_manga_record_worker(
        &mut manga_record_worker,
        config.manga_record_worker_addr,
        &config.manga_record_worker_token,
    ) {
        let _ = stop_manga_record_worker(&mut manga_record_worker);
        let _ = stop_video_record_worker(&mut video_record_worker);
        let _ = stop_character_mutation_worker(&mut character_mutation_worker);
        let _ = stop_config_read_worker(&mut config_read_worker);
        let _ = stop_workflow_read_worker(&mut workflow_read_worker);
        return Err(error);
    }
    let manga_record_worker_fingerprint = binary_fingerprint(&config.manga_record_worker_binary)?;
    let edge = match spawn_edge(&config.edge_binary, config.control_addr) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
            let _ = stop_project_mutation_worker(&mut project_mutation_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_manga_record_worker(&mut manga_record_worker);
            let _ = stop_video_record_worker(&mut video_record_worker);
            let _ = stop_character_mutation_worker(&mut character_mutation_worker);
            let _ = stop_config_read_worker(&mut config_read_worker);
            let _ = stop_workflow_read_worker(&mut workflow_read_worker);
            return Err(error);
        }
    };
    Ok(Runtime {
        config,
        project_read_worker_binary,
        edge,
        control,
        agent_worker,
        novel_planning_worker,
        artifact_media_worker,
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
        project_mutation_worker,
        chapter_scene_worker,
        chapter_content_worker,
        workflow_read_worker,
        config_read_worker,
        character_mutation_worker,
        video_record_worker,
        manga_record_worker,
        control_fingerprint,
        worker_fingerprint,
        edge_fingerprint,
        novel_planning_worker_fingerprint,
        artifact_media_worker_fingerprint,
        outline_worker_fingerprint,
        blueprint_worker_fingerprint,
        asset_graph_worker_fingerprint,
        foreshadowing_worker_fingerprint,
        object_profile_worker_fingerprint,
        scene_profile_worker_fingerprint,
        profile_material_worker_fingerprint,
        project_status_worker_fingerprint,
        portrait_worker_fingerprint,
        source_snapshot_worker_fingerprint,
        theme_read_worker_fingerprint,
        project_read_worker_fingerprint,
        import_review_read_worker_fingerprint,
        inspiration_history_worker_fingerprint,
        project_access_worker_fingerprint,
        project_collaboration_worker_fingerprint,
        project_mutation_worker_fingerprint,
        chapter_scene_worker_fingerprint,
        chapter_content_worker_fingerprint,
        workflow_read_worker_fingerprint,
        config_read_worker_fingerprint,
        character_mutation_worker_fingerprint,
        video_record_worker_fingerprint,
        manga_record_worker_fingerprint,
        reload_marker_fingerprint,
    })
}
