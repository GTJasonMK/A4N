use std::collections::HashSet;

use a4n_db_models::models::ai::system_settings;
use sea_orm::DatabaseConnection;

use super::identity::{parse_project_override_key, validate_project_override_project_id};
use super::storage::{
    MissingProjectMessage, all_prompt_override_settings, ensure_hydration_projects_exist,
    ensure_project_exists, project_override_record_content, project_prompt_override_settings,
};
use super::{ProjectOverrideHydration, ProjectPromptOverrideSummary, PromptOverrideDbError};

pub async fn project_override_hydration_plan(
    db: &DatabaseConnection,
) -> Result<Vec<ProjectOverrideHydration>, PromptOverrideDbError> {
    let records = all_prompt_override_settings(db).await?;
    let plan = project_override_hydration_candidates_from_records(&records)?;

    let project_ids = plan
        .iter()
        .map(|item| item.identity.project_id)
        .collect::<Vec<_>>();
    ensure_hydration_projects_exist(db, &project_ids).await?;

    Ok(plan)
}

pub async fn project_override_summaries(
    db: &DatabaseConnection,
    project_id: i64,
    active_prompt_names: &HashSet<String>,
) -> Result<Vec<ProjectPromptOverrideSummary>, PromptOverrideDbError> {
    let project_id = validate_project_override_project_id(project_id)?;
    ensure_project_exists(db, project_id, MissingProjectMessage::Command).await?;
    let records = project_prompt_override_settings(db, project_id).await?;

    project_override_summaries_from_records(project_id, records, active_prompt_names)
}

pub(super) fn project_override_hydration_candidates_from_records(
    records: &[system_settings::Model],
) -> Result<Vec<ProjectOverrideHydration>, PromptOverrideDbError> {
    let mut plan = Vec::new();

    for record in records {
        let identity = parse_project_override_key(&record.key)?;
        let content = project_override_record_content(record)?;
        plan.push(ProjectOverrideHydration { identity, content });
    }

    Ok(plan)
}

pub(super) fn project_override_summaries_from_records(
    project_id: i64,
    records: Vec<system_settings::Model>,
    active_prompt_names: &HashSet<String>,
) -> Result<Vec<ProjectPromptOverrideSummary>, PromptOverrideDbError> {
    let mut overrides = Vec::new();
    for record in records {
        let identity = parse_project_override_key(&record.key)?;
        if identity.project_id != project_id {
            continue;
        }
        let content = project_override_record_content(&record)?;
        if !active_prompt_names.contains(&identity.name) {
            continue;
        }
        overrides.push(ProjectPromptOverrideSummary {
            project_id,
            prompt_name: identity.name,
            content,
            updated_at: record.updated_at.to_string(),
        });
    }

    Ok(overrides)
}
