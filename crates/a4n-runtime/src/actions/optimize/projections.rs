use super::{
    OptimizationPatch, OptimizationPatchSetSummary, OptimizationSessionSummary,
    OptimizationTurnEventSummary, OptimizationTurnSummary,
};
use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_turn,
    optimization_turn_event,
};

pub(super) fn to_session_summary(
    model: &optimization_session::Model,
) -> OptimizationSessionSummary {
    OptimizationSessionSummary {
        id: model.id,
        project_id: model.project_id,
        chapter_id: model.chapter_id,
        name: model.name.clone(),
        status: model.status.clone(),
        issues: model.issues.clone(),
        agent_permissions: model.agent_permissions.clone(),
        agent_instruction: model.agent_instruction.clone(),
        fixes_applied: model.fixes_applied,
        summary: model.summary.clone(),
        created_at: Some(format_utc_timestamp(model.created_at)),
        updated_at: Some(format_utc_timestamp(model.updated_at)),
    }
}

pub(super) fn to_turn_summary(
    turn: &optimization_turn::Model,
    events: Vec<optimization_turn_event::Model>,
) -> OptimizationTurnSummary {
    OptimizationTurnSummary {
        id: turn.id,
        session_id: turn.session_id,
        user_text: turn.user_text.clone(),
        status: turn.status.clone(),
        permissions_snapshot: turn.permissions_snapshot.clone(),
        final_reply: turn.final_reply.clone(),
        error: turn.error.clone(),
        events: events.iter().map(to_turn_event_summary).collect(),
        created_at: Some(format_utc_timestamp(turn.created_at)),
        updated_at: Some(format_utc_timestamp(turn.updated_at)),
    }
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
        created_at: Some(format_utc_timestamp(model.created_at)),
    }
}

pub(super) fn to_patch_set_summary(
    patch_set: &optimization_patch_set::Model,
    patches: Vec<optimization_patch::Model>,
) -> OptimizationPatchSetSummary {
    OptimizationPatchSetSummary {
        id: patch_set.id,
        turn_id: patch_set.turn_id,
        session_id: patch_set.session_id,
        chapter_id: patch_set.chapter_id,
        base_content_hash: patch_set.base_content_hash.clone(),
        status: patch_set.status.clone(),
        summary: patch_set.summary.clone(),
        patches: patches
            .iter()
            .map(|patch| OptimizationPatch {
                patch_id: patch.patch_id.clone(),
                issue_id: patch.issue_id.clone(),
                operation: patch.operation.clone(),
                old_text: patch.old_text.clone(),
                new_text: patch.new_text.clone(),
                reason: patch.reason.clone(),
                risk: patch.risk.clone(),
            })
            .collect(),
        created_at: Some(format_utc_timestamp(patch_set.created_at)),
        updated_at: Some(format_utc_timestamp(patch_set.updated_at)),
    }
}

fn format_utc_timestamp(value: chrono::NaiveDateTime) -> String {
    value.and_utc().to_rfc3339()
}
