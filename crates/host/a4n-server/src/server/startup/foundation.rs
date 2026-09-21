use super::super::*;
use super::state::{Foundation, StartupConfig};

pub(crate) fn start(config: &StartupConfig) -> anyhow::Result<Foundation> {
    let mut artifact_media_worker = spawn_artifact_media_worker(
        &config.artifact_media_worker_binary,
        config.artifact_media_worker_addr,
        &config.artifact_media_worker_token,
    )?;
    if let Err(error) = wait_for_artifact_media_worker(
        &mut artifact_media_worker,
        config.artifact_media_worker_addr,
        &config.artifact_media_worker_token,
    ) {
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut novel_planning_worker = match spawn_novel_planning_worker(
        &config.novel_planning_worker_binary,
        config.novel_planning_worker_addr,
        &config.novel_planning_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_novel_planning_worker(
        &mut novel_planning_worker,
        config.novel_planning_worker_addr,
        &config.novel_planning_worker_token,
    ) {
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut outline_worker = match spawn_outline_worker(
        &config.outline_worker_binary,
        config.outline_worker_addr,
        &config.outline_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_outline_worker(
        &mut outline_worker,
        config.outline_worker_addr,
        &config.outline_worker_token,
    ) {
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut blueprint_worker = match spawn_blueprint_worker(
        &config.blueprint_worker_binary,
        config.blueprint_worker_addr,
        &config.blueprint_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_blueprint_worker(
        &mut blueprint_worker,
        config.blueprint_worker_addr,
        &config.blueprint_worker_token,
    ) {
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut asset_graph_worker = match spawn_asset_graph_worker(
        &config.asset_graph_worker_binary,
        config.asset_graph_worker_addr,
        &config.asset_graph_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_asset_graph_worker(
        &mut asset_graph_worker,
        config.asset_graph_worker_addr,
        &config.asset_graph_worker_token,
    ) {
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut foreshadowing_worker = match spawn_foreshadowing_worker(
        &config.foreshadowing_worker_binary,
        config.foreshadowing_worker_addr,
        &config.foreshadowing_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_foreshadowing_worker(
        &mut foreshadowing_worker,
        config.foreshadowing_worker_addr,
        &config.foreshadowing_worker_token,
    ) {
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut object_profile_worker = match spawn_object_profile_worker(
        &config.object_profile_worker_binary,
        config.object_profile_worker_addr,
        &config.object_profile_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_object_profile_worker(
        &mut object_profile_worker,
        config.object_profile_worker_addr,
        &config.object_profile_worker_token,
    ) {
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut scene_profile_worker = match spawn_scene_profile_worker(
        &config.scene_profile_worker_binary,
        config.scene_profile_worker_addr,
        &config.scene_profile_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_scene_profile_worker(
        &mut scene_profile_worker,
        config.scene_profile_worker_addr,
        &config.scene_profile_worker_token,
    ) {
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut portrait_worker = match spawn_portrait_worker(
        &config.portrait_worker_binary,
        config.portrait_worker_addr,
        &config.portrait_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_portrait_worker(
        &mut portrait_worker,
        config.portrait_worker_addr,
        &config.portrait_worker_token,
    ) {
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut source_snapshot_worker = match spawn_source_snapshot_worker(
        &config.source_snapshot_worker_binary,
        config.source_snapshot_worker_addr,
        &config.source_snapshot_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_source_snapshot_worker(
        &mut source_snapshot_worker,
        config.source_snapshot_worker_addr,
        &config.source_snapshot_worker_token,
    ) {
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut config_read_worker = match spawn_config_read_worker(
        &config.config_read_worker_binary,
        config.config_read_worker_addr,
        &config.config_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_config_read_worker(
        &mut config_read_worker,
        config.config_read_worker_addr,
        &config.config_read_worker_token,
    ) {
        let _ = stop_config_read_worker(&mut config_read_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let config_read_worker_fingerprint = binary_fingerprint(&config.config_read_worker_binary)?;
    Ok(Foundation {
        artifact_media_worker,
        novel_planning_worker,
        outline_worker,
        blueprint_worker,
        asset_graph_worker,
        foreshadowing_worker,
        object_profile_worker,
        scene_profile_worker,
        portrait_worker,
        source_snapshot_worker,
        config_read_worker,
        control_fingerprint: config.control_fingerprint,
        worker_fingerprint: config.worker_fingerprint,
        edge_fingerprint: config.edge_fingerprint,
        novel_planning_worker_fingerprint: config.novel_planning_worker_fingerprint,
        artifact_media_worker_fingerprint: config.artifact_media_worker_fingerprint,
        outline_worker_fingerprint: config.outline_worker_fingerprint,
        blueprint_worker_fingerprint: config.blueprint_worker_fingerprint,
        asset_graph_worker_fingerprint: config.asset_graph_worker_fingerprint,
        foreshadowing_worker_fingerprint: config.foreshadowing_worker_fingerprint,
        object_profile_worker_fingerprint: config.object_profile_worker_fingerprint,
        scene_profile_worker_fingerprint: config.scene_profile_worker_fingerprint,
        profile_material_worker_fingerprint: config.profile_material_worker_fingerprint,
        project_status_worker_fingerprint: config.project_status_worker_fingerprint,
        portrait_worker_fingerprint: config.portrait_worker_fingerprint,
        source_snapshot_worker_fingerprint: config.source_snapshot_worker_fingerprint,
        config_read_worker_fingerprint,
    })
}
