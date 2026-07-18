use super::{RollbackPreviewSummary, rules};
use crate::{AppState, actions::access::verify_project_access_string};
use a4n_core::state::validator::StateValidator;

pub async fn preview_rollback(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target_status: &str,
) -> Result<RollbackPreviewSummary, String> {
    verify_project_access_string(state, user_id, project_id).await?;
    let project = rules::project_record_by_id(state, project_id).await?;
    rules::ensure_text_project_status_command(&project)?;
    let from = StateValidator::parse_status(&project.status).map_err(|error| error.to_string())?;
    let to = StateValidator::parse_status(target_status).map_err(|error| error.to_string())?;
    StateValidator::validate_rollback(&project.status, target_status)
        .map_err(|error| error.to_string())?;

    let mut path = vec![project.status.clone()];
    path.extend(rules::rollback_path(from, to));
    let impacts = rules::build_rollback_impacts(state, project_id, from, to).await?;
    let affected_chapters = rules::rollback_impact_count(&impacts, "delete_chapters")?;
    let affected_outlines = rules::rollback_impact_count(&impacts, "delete_chapter_outlines")?;
    let summary = rules::build_rollback_summary(&path, &impacts);
    let warning_message = if impacts.is_empty() {
        None
    } else {
        Some("回退会删除依赖数据，此操作不可恢复。".to_string())
    };

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
