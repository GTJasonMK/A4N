use crate::source_sync::SourceSyncDbError;
use a4n_core::models::source_sync::source_sync_conflict;
use a4n_domain::source_sync::SourceSyncLocalStateKind;
use serde_json::Value;

pub(super) fn ensure_clean_source_sync_target(
    conflict: &source_sync_conflict::Model,
    strategy: &str,
) -> Result<(), SourceSyncDbError> {
    if SourceSyncLocalStateKind::parse(&conflict.local_state_kind).is_some_and(|state| {
        matches!(
            state,
            SourceSyncLocalStateKind::Clean | SourceSyncLocalStateKind::Draft
        )
    }) {
        Ok(())
    } else {
        Err(SourceSyncDbError::InvalidArgument(format!(
            "{strategy} 不能处理本地状态为 {} 的对象，请使用保留当前、手动合并或创建替换版本",
            conflict.local_state_kind
        )))
    }
}

pub(super) fn ensure_source_sync_decision_targets_generation_spec(
    conflict: &source_sync_conflict::Model,
    generation_spec_id: i64,
    strategy: &str,
) -> Result<(), SourceSyncDbError> {
    let expected = source_sync_generation_spec_id_for_conflict(conflict, strategy)?;
    if generation_spec_id == expected {
        Ok(())
    } else {
        Err(SourceSyncDbError::InvalidArgument(format!(
            "{strategy} 决策目标生成意图不匹配：冲突 {} 影响 generation_spec={}，payload 指向 generation_spec={}",
            conflict.id, expected, generation_spec_id
        )))
    }
}

pub(super) fn source_sync_generation_spec_id_for_conflict(
    conflict: &source_sync_conflict::Model,
    strategy: &str,
) -> Result<i64, SourceSyncDbError> {
    if conflict.conflict_scope == "generation_spec" {
        return Ok(conflict.target_id);
    }
    if conflict.conflict_scope == "text_anchor" && conflict.target_kind == "segment" {
        return Ok(conflict.target_id);
    }
    if let Some(generation_spec_id) = conflict
        .local_summary_json
        .get("generation_spec_id")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
    {
        return Ok(generation_spec_id);
    }
    Err(SourceSyncDbError::InvalidArgument(format!(
        "{strategy} 决策要求冲突绑定生成意图，但冲突 {} 未提供 generation_spec_id",
        conflict.id
    )))
}
