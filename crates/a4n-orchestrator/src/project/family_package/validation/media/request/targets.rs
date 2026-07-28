use a4n_core::models::media::media_asset;

use super::super::contracts::MediaRequestRefContext;
use super::galgame::validate_media_request_galgame_scene_project;

pub(super) fn validate_media_request_target_entity(
    media_asset_id: i64,
    asset: &media_asset::Model,
    target_entity_kind: &str,
    target_entity_id: i64,
    refs: &MediaRequestRefContext<'_>,
) -> Result<(), String> {
    match target_entity_kind {
        "character_profile" => {
            if !refs.character_profile_ids.contains(&target_entity_id) {
                return Err(format!(
                    "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的角色档案不存在: media_asset_id={}, target_entity_id={}",
                    media_asset_id, target_entity_id
                ));
            }
        }
        "segment" => {
            if !refs.video_segment_ids.contains(&target_entity_id) {
                return Err(format!(
                    "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的视频片段不存在: media_asset_id={}, target_entity_id={}",
                    media_asset_id, target_entity_id
                ));
            }
        }
        "assembly" => {
            if !refs.assembly_timeline_ids.contains(&target_entity_id) {
                return Err(format!(
                    "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的成片时间线不存在: media_asset_id={}, target_entity_id={}",
                    media_asset_id, target_entity_id
                ));
            }
        }
        "galgame_scene" => {
            let scene = refs
                .galgame_scenes
                .get(&target_entity_id)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的 Galgame 场不存在: media_asset_id={}, target_entity_id={target_entity_id}",
                        media_asset_id
                    )
                })?;
            validate_media_request_galgame_scene_project(
                media_asset_id,
                asset,
                scene,
                refs.galgame_scripts,
            )?;
        }
        "galgame_line" => {
            let line = refs
                .galgame_lines
                .get(&target_entity_id)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "项目族导入包媒体来源 request_snapshot_json.target_entity_id 引用的 Galgame 行不存在: media_asset_id={}, target_entity_id={target_entity_id}",
                        media_asset_id
                    )
                })?;
            let scene = refs
                .galgame_scenes
                .get(&line.scene_id)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "项目族导入包媒体来源 request_snapshot_json.target_entity_id 行所属场不存在: media_asset_id={}, target_entity_id={}, scene_id={}",
                        media_asset_id, target_entity_id, line.scene_id
                    )
                })?;
            validate_media_request_galgame_scene_project(
                media_asset_id,
                asset,
                scene,
                refs.galgame_scripts,
            )?;
        }
        _ => {}
    }
    Ok(())
}
