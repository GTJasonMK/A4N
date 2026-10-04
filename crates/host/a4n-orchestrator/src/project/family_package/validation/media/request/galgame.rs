use std::collections::HashMap;

pub(super) fn validate_media_request_galgame_target_line(
    media_asset_id: i64,
    asset: &a4n_domain::project::ProjectFamilyMediaAssetRecord,
    target_line_id: i64,
    galgame_scripts: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameScriptRecord>,
    galgame_scenes: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameSceneRecord>,
    galgame_lines: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameLineRecord>,
) -> Result<(), String> {
    let line = galgame_lines.get(&target_line_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.target_line_id 引用的 Galgame 行不存在: media_asset_id={}, target_line_id={target_line_id}",
            media_asset_id
        )
    })?;
    let scene = galgame_scenes.get(&line.scene_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json.target_line_id 所属场不存在: media_asset_id={}, target_line_id={target_line_id}, scene_id={}",
            media_asset_id, line.scene_id
        )
    })?;
    validate_media_request_galgame_scene_project(media_asset_id, asset, scene, galgame_scripts)
}

pub(super) fn validate_media_request_galgame_scene_project(
    media_asset_id: i64,
    asset: &a4n_domain::project::ProjectFamilyMediaAssetRecord,
    scene: &a4n_domain::project::ProjectFamilyGalgameSceneRecord,
    galgame_scripts: &HashMap<i64, &a4n_domain::project::ProjectFamilyGalgameScriptRecord>,
) -> Result<(), String> {
    let script = galgame_scripts.get(&scene.script_id).copied().ok_or_else(|| {
        format!(
            "项目族导入包媒体来源 request_snapshot_json Galgame 场引用了不存在的剧本: media_asset_id={}, scene_id={}, script_id={}",
            media_asset_id, scene.id, scene.script_id
        )
    })?;
    if asset.project_id != script.project_id || asset.root_project_id != script.root_project_id {
        return Err(format!(
            "项目族导入包媒体来源 request_snapshot_json Galgame 目标与媒体资产项目不一致: media_asset_id={}, asset_project_id={}, target_project_id={}",
            media_asset_id, asset.project_id, script.project_id
        ));
    }
    Ok(())
}
