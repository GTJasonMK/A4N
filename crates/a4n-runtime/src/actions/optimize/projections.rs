use super::{
    OptimizationAgentPermissions, OptimizationPatch, OptimizationPatchOperation,
    OptimizationPatchSetSummary, OptimizationSessionSummary, OptimizationTurnEventSummary,
    OptimizationTurnSummary,
};
use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_turn,
    optimization_turn_event,
};

pub(super) fn to_session_summary(
    model: &optimization_session::Model,
) -> Result<OptimizationSessionSummary, String> {
    Ok(OptimizationSessionSummary {
        id: model.id,
        project_id: model.project_id,
        chapter_id: model.chapter_id,
        name: model.name.clone(),
        status: model.status.clone(),
        issues: model.issues.clone(),
        agent_permissions: required_permissions(
            model.agent_permissions.as_ref(),
            "optimization_session.agent_permissions",
        )?,
        agent_instruction: model.agent_instruction.clone(),
        fixes_applied: model.fixes_applied,
        summary: model.summary.clone(),
        created_at: format_utc_timestamp(model.created_at),
        updated_at: format_utc_timestamp(model.updated_at),
    })
}

pub(super) fn to_turn_summary(
    turn: &optimization_turn::Model,
    events: Vec<optimization_turn_event::Model>,
) -> Result<OptimizationTurnSummary, String> {
    Ok(OptimizationTurnSummary {
        id: turn.id,
        session_id: turn.session_id,
        user_text: turn.user_text.clone(),
        status: turn.status.clone(),
        permissions_snapshot: required_permissions(
            turn.permissions_snapshot.as_ref(),
            "optimization_turn.permissions_snapshot",
        )?,
        final_reply: turn.final_reply.clone(),
        error: turn.error.clone(),
        events: events.iter().map(to_turn_event_summary).collect(),
        created_at: format_utc_timestamp(turn.created_at),
        updated_at: format_utc_timestamp(turn.updated_at),
    })
}

fn to_turn_event_summary(model: &optimization_turn_event::Model) -> OptimizationTurnEventSummary {
    OptimizationTurnEventSummary {
        id: model.id,
        turn_id: model.turn_id,
        seq: model.seq,
        kind: model.kind.clone(),
        status: model.status.clone(),
        public_text: model.public_text.clone(),
        payload: model.payload.clone(),
        created_at: format_utc_timestamp(model.created_at),
    }
}

pub(super) fn to_patch_set_summary(
    patch_set: &optimization_patch_set::Model,
    patches: Vec<optimization_patch::Model>,
) -> Result<OptimizationPatchSetSummary, String> {
    Ok(OptimizationPatchSetSummary {
        id: patch_set.id,
        turn_id: patch_set.turn_id,
        session_id: patch_set.session_id,
        chapter_id: patch_set.chapter_id,
        base_content_hash: patch_set.base_content_hash.clone(),
        status: patch_set.status.clone(),
        summary: patch_set.summary.clone(),
        patches: patches
            .iter()
            .map(|patch| -> Result<OptimizationPatch, String> {
                Ok(OptimizationPatch {
                    patch_id: patch.patch_id.clone(),
                    issue_id: patch.issue_id.clone(),
                    operation: parse_patch_operation(&patch.operation)?,
                    old_text: patch.old_text.clone(),
                    new_text: patch.new_text.clone(),
                    reason: patch.reason.clone(),
                    risk: patch.risk.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        created_at: format_utc_timestamp(patch_set.created_at),
        updated_at: format_utc_timestamp(patch_set.updated_at),
    })
}

fn format_utc_timestamp(value: chrono::NaiveDateTime) -> String {
    value.and_utc().to_rfc3339()
}

fn required_permissions(
    value: Option<&serde_json::Value>,
    field: &str,
) -> Result<OptimizationAgentPermissions, String> {
    serde_json::from_value(value.cloned().ok_or_else(|| format!("{field} 缺失"))?)
        .map_err(|error| format!("{field} 无效: {error}"))
}

fn parse_patch_operation(value: &str) -> Result<OptimizationPatchOperation, String> {
    match value {
        "replace" => Ok(OptimizationPatchOperation::Replace),
        "delete" => Ok(OptimizationPatchOperation::Delete),
        "insert_before" => Ok(OptimizationPatchOperation::InsertBefore),
        "insert_after" => Ok(OptimizationPatchOperation::InsertAfter),
        _ => Err(format!("optimization_patch.operation 无效: {value}")),
    }
}
