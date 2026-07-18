use a4n_core::models::project::novel_project;

use crate::state::AppState;

pub(super) async fn load_project_family_export_selection(
    state: &AppState,
    root_project_id: i64,
    include_derived_projects: bool,
) -> Result<(novel_project::Model, Vec<novel_project::Model>), String> {
    let selection = state
        .orchestrator
        .project_family_export_selection(root_project_id, include_derived_projects)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok((selection.root, selection.derived_projects))
}

pub(super) async fn load_derived_project_export_selection(
    state: &AppState,
    derived_project_id: i64,
) -> Result<(novel_project::Model, Vec<novel_project::Model>), String> {
    let selection = state
        .orchestrator
        .derived_project_export_selection(derived_project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok((selection.root, selection.derived_projects))
}

pub(super) fn validate_same_user(
    root: &novel_project::Model,
    derived_projects: &[novel_project::Model],
) -> Result<(), String> {
    for project in derived_projects {
        if project.user_id != root.user_id {
            return Err(format!(
                "派生项目所属用户与根文本项目不一致: {}",
                project.id
            ));
        }
        if project.root_project_id != Some(root.id) {
            return Err(format!("派生项目不属于根文本项目: {}", project.id));
        }
    }
    Ok(())
}
