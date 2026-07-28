use super::super::contracts::SourceSyncReferenceSets;
use a4n_core::models::source_sync::source_sync_conflict;
use serde_json::Value;

pub(in super::super) fn validate_source_sync_conflict_target_ref(
    conflict: &source_sync_conflict::Model,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let exists = match (
        conflict.conflict_scope.as_str(),
        conflict.target_kind.as_str(),
    ) {
        ("generation_spec", "segment") | ("text_anchor", "segment") => {
            refs.generation_spec_ids.contains(&conflict.target_id)
        }
        ("text_anchor", "chapter") => refs.chapter_ids.contains(&conflict.target_id),
        ("text_anchor", "galgame_scene") => refs.galgame_scene_ids.contains(&conflict.target_id),
        ("text_anchor", "galgame_line") => refs.galgame_line_ids.contains(&conflict.target_id),
        ("profile", "character") => refs.character_profile_ids.contains(&conflict.target_id),
        ("profile", "object") => refs.object_profile_ids.contains(&conflict.target_id),
        ("profile", "scene") => refs.scene_profile_ids.contains(&conflict.target_id),
        _ => return Ok(()),
    };
    ensure_package!(
        exists,
        "项目族导入包源同步冲突目标不存在: source_sync_conflict_id={}, conflict_scope={}, target_kind={}, target_id={}",
        conflict.id,
        conflict.conflict_scope,
        conflict.target_kind,
        conflict.target_id
    );
    Ok(())
}

pub(in super::super) fn validate_source_sync_conflict_regenerate_scene_target(
    conflict: &source_sync_conflict::Model,
    strategy: &str,
    payload: &Value,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    if strategy != "regenerate_scene" {
        return Ok(());
    }
    ensure_package!(
        conflict.conflict_scope == "text_anchor"
            && matches!(
                conflict.target_kind.as_str(),
                "galgame_scene" | "galgame_line"
            ),
        "项目族导入包 regenerate_scene 只能用于 Galgame 场/行文本锚点冲突: source_sync_conflict_id={}, conflict_scope={}, target_kind={}",
        conflict.id,
        conflict.conflict_scope,
        conflict.target_kind
    );
    let payload_scene_id = payload
        .get("galgame_scene_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene galgame_scene_id 必须是整数: source_sync_conflict_id={}",
                conflict.id
            )
        })?;
    let expected_scene_id = match conflict.target_kind.as_str() {
        "galgame_scene" => conflict.target_id,
        "galgame_line" => refs
            .galgame_line_scene_ids
            .get(&conflict.target_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene 冲突目标 Galgame 行不存在: source_sync_conflict_id={}, galgame_line_id={}",
                    conflict.id, conflict.target_id
                )
            })?,
        other => {
            return Err(format!(
                "项目族导入包 regenerate_scene 冲突目标类型不受支持: source_sync_conflict_id={}, target_kind={other}",
                conflict.id
            ));
        }
    };
    ensure_package!(
        payload_scene_id == expected_scene_id,
        "项目族导入包 regenerate_scene galgame_scene_id 与冲突目标场不一致: source_sync_conflict_id={}, galgame_scene_id={}, expected={}",
        conflict.id,
        payload_scene_id,
        expected_scene_id
    );
    Ok(())
}
