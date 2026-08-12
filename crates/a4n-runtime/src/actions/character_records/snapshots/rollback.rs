use super::super::access::require_profile_for_user;
use super::super::contracts::{RollbackResultSummary, RollbackToChapterInput};
use crate::state::AppState;
use a4n_orchestrator::character::CharacterTimelineRollbackLifecycleResult;

pub async fn rollback_to_chapter(
    state: &AppState,
    user_id: i64,
    input: RollbackToChapterInput,
) -> Result<RollbackResultSummary, String> {
    if input.chapter_number < 0 {
        return Err("回滚章节号不能小于 0".to_string());
    }
    let profile = require_profile_for_user(state, user_id, input.profile_id).await?;
    let trace_id = crate::sidecar_clients::character_profile_rollback_rag_operation_trace_id(
        profile.project_id,
        input.profile_id,
        input.chapter_number,
    )?;
    let result = crate::sidecar_clients::scope_rag_operation_trace(trace_id, async {
        state
            .orchestrator()
            .rollback_character_profile_timeline_validated(input.profile_id, input.chapter_number)
            .await
            .map_err(|error| error.to_string())
    })
    .await?;
    let CharacterTimelineRollbackLifecycleResult { rollback, rag } = result;
    let a4n_orchestrator::rag::CharacterTimelineRagDeletionSummary {
        deleted_state_rag,
        deleted_attribute_event_rag,
    } = rag;

    Ok(RollbackResultSummary {
        deleted_events: rollback.deleted_timeline.deleted_attribute_events,
        deleted_snapshots: rollback.deleted_timeline.deleted_snapshots,
        deleted_behaviors: rollback.deleted_timeline.deleted_behaviors,
        deleted_state_indexes: rollback.deleted_timeline.deleted_state_indexes,
        deleted_state_rag,
        deleted_attribute_event_rag,
        deleted_deletion_marks: rollback.deletion_mark_rewind.deleted,
        rewound_deletion_marks: rollback.deletion_mark_rewind.rewound,
    })
}
