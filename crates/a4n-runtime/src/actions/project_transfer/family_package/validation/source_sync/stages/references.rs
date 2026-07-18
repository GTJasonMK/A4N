use serde_json::{Map, Value};

use super::super::contracts::{RegenerateSceneStageOutputContext, SourceSyncReferenceSets};
use super::super::json::{validate_optional_json_id_ref, validate_source_span_ids};
use super::shape::{
    validate_regenerate_scene_branch_design_stage_output,
    validate_regenerate_scene_single_scene_line_stage_output,
    validate_regenerate_scene_visual_planning_stage_output,
    validate_regenerate_shooting_script_projection_stage_output,
};

pub(super) fn validate_stage_output_object(
    object: &Map<String, Value>,
    refs: &SourceSyncReferenceSets,
    context: &RegenerateSceneStageOutputContext,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    validate_regenerate_shooting_script_projection_stage_output(
        object, owner_kind, owner_id, path,
    )?;
    validate_regenerate_scene_single_scene_line_stage_output(object, owner_kind, owner_id, path)?;
    validate_regenerate_scene_visual_planning_stage_output(object, owner_kind, owner_id, path)?;
    validate_regenerate_scene_branch_design_stage_output(object, owner_kind, owner_id, path)?;
    if let Some(text_anchor_id) = validate_optional_json_id_ref(
        object,
        "text_anchor_id",
        &refs.text_anchor_ids,
        "文本锚点",
        owner_kind,
        owner_id,
    )? {
        let anchor_chapter_id = refs
            .text_anchor_source_snapshot_chapter_ids
            .get(&text_anchor_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene {path}.text_anchor_id 不属于源快照章节: {owner_kind}_id={}, text_anchor_id={}",
                    owner_id, text_anchor_id
                )
            })?;
        ensure_package!(
            anchor_chapter_id == context.expected_source_snapshot_chapter_id,
            "项目族导入包 regenerate_scene {path}.text_anchor_id 不属于重建目标源快照章节: {owner_kind}_id={}, text_anchor_id={}, source_snapshot_chapter_id={}, expected={}",
            owner_id,
            text_anchor_id,
            anchor_chapter_id,
            context.expected_source_snapshot_chapter_id
        );
    }
    validate_optional_json_id_ref(
        object,
        "speaker_profile_id",
        &refs.character_profile_ids,
        "角色档案",
        owner_kind,
        owner_id,
    )?;
    validate_optional_json_id_ref(
        object,
        "scene_profile_id",
        &refs.scene_profile_ids,
        "场景档案",
        owner_kind,
        owner_id,
    )?;
    if let Some(source_snapshot_chapter_id) = validate_optional_json_id_ref(
        object,
        "source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        owner_kind,
        owner_id,
    )? {
        ensure_package!(
            source_snapshot_chapter_id == context.expected_source_snapshot_chapter_id,
            "项目族导入包 regenerate_scene {path}.source_snapshot_chapter_id 与重建目标章节不一致: {owner_kind}_id={}, source_snapshot_chapter_id={}, expected={}",
            owner_id,
            source_snapshot_chapter_id,
            context.expected_source_snapshot_chapter_id
        );
    }
    if let Some(source_chapter_id) = validate_optional_json_id_ref(
        object,
        "source_chapter_id",
        &refs.chapter_ids,
        "源章节",
        owner_kind,
        owner_id,
    )? {
        ensure_package!(
            source_chapter_id == context.expected_source_chapter_id,
            "项目族导入包 regenerate_scene {path}.source_chapter_id 与重建目标源章节不一致: {owner_kind}_id={}, source_chapter_id={}, expected={}",
            owner_id,
            source_chapter_id,
            context.expected_source_chapter_id
        );
    }
    if let Some(galgame_scene_id) = validate_optional_json_id_ref(
        object,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        owner_kind,
        owner_id,
    )? {
        ensure_package!(
            galgame_scene_id == context.expected_galgame_scene_id,
            "项目族导入包 regenerate_scene {path}.galgame_scene_id 与重建目标场不一致: {owner_kind}_id={}, galgame_scene_id={}, expected={}",
            owner_id,
            galgame_scene_id,
            context.expected_galgame_scene_id
        );
    }
    if let Some(galgame_line_id) = validate_optional_json_id_ref(
        object,
        "galgame_line_id",
        &refs.galgame_line_ids,
        "Galgame 行",
        owner_kind,
        owner_id,
    )? {
        let line_scene_id = refs
            .galgame_line_scene_ids
            .get(&galgame_line_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene {path}.galgame_line_id 引用的 Galgame 行不存在: {owner_kind}_id={}, galgame_line_id={}",
                    owner_id, galgame_line_id
                )
            })?;
        ensure_package!(
            line_scene_id == context.expected_galgame_scene_id,
            "项目族导入包 regenerate_scene {path}.galgame_line_id 不属于重建目标场: {owner_kind}_id={}, galgame_line_id={}, scene_id={}, expected={}",
            owner_id,
            galgame_line_id,
            line_scene_id,
            context.expected_galgame_scene_id
        );
    }
    validate_source_span_ids(object, refs, context, owner_kind, owner_id, path)
}
