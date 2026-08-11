mod batch;
mod projection;
mod single;

use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::project::novel_project;

pub use batch::project_summaries_with_family_for_user;
pub use single::project_summary_with_family_for_user;

pub(super) async fn project_record_by_id(
    state: &AppState,
    project_id: i64,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator()
        .project_by_id(project_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("项目不存在: {project_id}"))
}
