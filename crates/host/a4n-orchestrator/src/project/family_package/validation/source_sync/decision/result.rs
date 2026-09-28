use serde_json::Value;

use super::super::contracts::{RegenerateSceneStageOutputContext, SourceSyncReferenceSets};
use super::super::json::{
    validate_json_object_keys, validate_required_json_string_value,
    validate_required_non_negative_json_count, validate_required_result_json_id_ref,
};
use super::super::stages::validate_galgame_stage_outputs;

pub(super) fn validate_regenerate_scene_result(
    result_value: Option<&Value>,
    expected_galgame_scene_id: i64,
    refs: &SourceSyncReferenceSets,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let result_value = result_value.ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result 缺失: {owner_kind}_id={owner_id}")
    })?;
    let result = result_value.as_object().ok_or_else(|| {
        format!("项目族导入包 regenerate_scene result 必须是 JSON 对象: {owner_kind}_id={owner_id}")
    })?;
    validate_json_object_keys(
        result,
        &[
            "galgame_scene_id",
            "old_source_snapshot_chapter_id",
            "new_source_snapshot_chapter_id",
            "old_line_count",
            "new_line_count",
            "deleted_generation_spec_count",
            "deleted_portrait_generation_spec_count",
            "resolved_related_conflict_count",
            "mode",
            "stage_outputs",
        ],
        "regenerate_scene result",
        owner_kind,
        owner_id,
    )?;
    let result_scene_id = validate_required_result_json_id_ref(
        result,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        owner_kind,
        owner_id,
    )?;
    ensure_package!(
        result_scene_id == expected_galgame_scene_id,
        "项目族导入包 regenerate_scene result.galgame_scene_id 与决策目标场不一致: {owner_kind}_id={}, galgame_scene_id={}, expected={}",
        owner_id,
        result_scene_id,
        expected_galgame_scene_id
    );
    validate_required_result_json_id_ref(
        result,
        "old_source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        owner_kind,
        owner_id,
    )?;
    let new_source_snapshot_chapter_id = validate_required_result_json_id_ref(
        result,
        "new_source_snapshot_chapter_id",
        &refs.source_snapshot_chapter_ids,
        "源快照章节",
        owner_kind,
        owner_id,
    )?;
    let actual_scene_snapshot_chapter_id = refs
        .galgame_scene_source_snapshot_chapter_ids
        .get(&expected_galgame_scene_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene galgame_scene_id 引用的 Galgame 场不存在: {owner_kind}_id={}, galgame_scene_id={}",
                owner_id, expected_galgame_scene_id
            )
        })?;
    ensure_package!(
        actual_scene_snapshot_chapter_id == new_source_snapshot_chapter_id,
        "项目族导入包 regenerate_scene result.new_source_snapshot_chapter_id 与 Galgame 场当前源快照章节不一致: {owner_kind}_id={}, galgame_scene_id={}, scene_source_snapshot_chapter_id={}, result_new_source_snapshot_chapter_id={}",
        owner_id,
        expected_galgame_scene_id,
        actual_scene_snapshot_chapter_id,
        new_source_snapshot_chapter_id
    );
    validate_required_non_negative_json_count(result, "old_line_count", owner_kind, owner_id)?;
    validate_required_non_negative_json_count(result, "new_line_count", owner_kind, owner_id)?;
    validate_required_non_negative_json_count(
        result,
        "deleted_generation_spec_count",
        owner_kind,
        owner_id,
    )?;
    validate_required_non_negative_json_count(
        result,
        "deleted_portrait_generation_spec_count",
        owner_kind,
        owner_id,
    )?;
    validate_required_non_negative_json_count(
        result,
        "resolved_related_conflict_count",
        owner_kind,
        owner_id,
    )?;
    validate_required_json_string_value(
        result,
        "mode",
        "llm_partial_regeneration",
        "regenerate_scene result",
        owner_kind,
        owner_id,
    )?;
    let stage_outputs = result.get("stage_outputs").ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene result.stage_outputs 缺失: {owner_kind}_id={owner_id}"
        )
    })?;
    if !stage_outputs.is_object() {
        return Err(format!(
            "项目族导入包 regenerate_scene result.stage_outputs 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        ));
    }
    if a4n_domain::media_authoring::legacy::source_sync::archived_stage_outputs(stage_outputs)? {
        return Ok(());
    }
    let (expected_source_chapter_id, expected_chapter_number) = refs
        .source_snapshot_chapter_contexts
        .get(&new_source_snapshot_chapter_id)
        .copied()
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene result.new_source_snapshot_chapter_id 引用的源快照章节不存在: {owner_kind}_id={}, new_source_snapshot_chapter_id={}",
                owner_id, new_source_snapshot_chapter_id
            )
        })?;
    let context = RegenerateSceneStageOutputContext {
        expected_galgame_scene_id,
        expected_source_snapshot_chapter_id: new_source_snapshot_chapter_id,
        expected_source_chapter_id,
        expected_chapter_number,
    };
    validate_galgame_stage_outputs(
        stage_outputs,
        refs,
        &context,
        owner_kind,
        owner_id,
        "result.stage_outputs",
    )?;
    crate::galgame::validate_source_sync_scene_regeneration_stage_outputs(stage_outputs)
        .map_err(|error| format!("项目族导入包 {owner_kind}_id={owner_id}: {error}"))?;
    Ok(())
}
