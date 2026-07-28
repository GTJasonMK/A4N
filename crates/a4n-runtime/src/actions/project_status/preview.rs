use super::{RollbackPreviewSummary, host_adapter};
use crate::{AppState, actions::access::verify_project_access_string};
use a4n_core::state::validator::StateValidator;
use a4n_orchestrator::project::{
    build_rollback_summary, ensure_text_project_status_command, rollback_impact_count,
    rollback_preview_path, rollback_preview_warning_message,
};

pub async fn preview_rollback(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target_status: &str,
) -> Result<RollbackPreviewSummary, String> {
    verify_project_access_string(state, user_id, project_id).await?;
    let project = host_adapter::project_record_by_id(state, project_id).await?;
    ensure_text_project_status_command(&project)?;
    let from = StateValidator::parse_status(&project.status).map_err(|error| error.to_string())?;
    let to = StateValidator::parse_status(target_status).map_err(|error| error.to_string())?;
    StateValidator::validate_rollback(&project.status, target_status)
        .map_err(|error| error.to_string())?;

    let path = rollback_preview_path(project.status.clone(), from, to);
    let impacts = host_adapter::build_rollback_impacts(state, project_id, from, to).await?;
    let affected_chapters = rollback_impact_count(&impacts, "delete_chapters")?;
    let affected_outlines = rollback_impact_count(&impacts, "delete_chapter_outlines")?;
    let summary = build_rollback_summary(&path, &impacts);
    let warning_message = rollback_preview_warning_message(&impacts);

    Ok(RollbackPreviewSummary {
        from_status: project.status,
        target_status: target_status.to_string(),
        path,
        affected_chapters,
        affected_outlines,
        can_rollback: true,
        warning_message,
        summary,
        impacts,
    })
}
