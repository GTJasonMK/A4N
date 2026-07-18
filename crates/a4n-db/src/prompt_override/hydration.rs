use std::collections::HashSet;

use a4n_core::models::ai::system_settings;
use sea_orm::DatabaseConnection;

use super::identity::{parse_project_override_key, validate_project_override_project_id};
use super::storage::{
    MissingProjectMessage, all_system_settings, ensure_project_exists,
    project_override_record_content,
};
use super::types::invalid_argument;
use super::{ProjectOverrideHydration, ProjectPromptOverrideSummary, PromptOverrideDbError};

pub async fn project_override_hydration_plan(
    db: &DatabaseConnection,
) -> Result<Vec<ProjectOverrideHydration>, PromptOverrideDbError> {
    let records = all_system_settings(db).await?;
    let plan = project_override_hydration_candidates_from_records(&records)?;

    for item in &plan {
        ensure_project_exists(db, item.identity.project_id, MissingProjectMessage::Hydrate).await?;
    }

    Ok(plan)
}

pub async fn project_override_summaries(
    db: &DatabaseConnection,
    project_id: i64,
    active_prompt_names: &HashSet<String>,
) -> Result<Vec<ProjectPromptOverrideSummary>, PromptOverrideDbError> {
    let project_id = validate_project_override_project_id(project_id)?;
    ensure_project_exists(db, project_id, MissingProjectMessage::Command).await?;
    let records = all_system_settings(db).await?;

    project_override_summaries_from_records(project_id, records, active_prompt_names)
}

pub(super) fn project_override_hydration_candidates_from_records(
    records: &[system_settings::Model],
) -> Result<Vec<ProjectOverrideHydration>, PromptOverrideDbError> {
    let mut plan = Vec::new();

    for record in records {
        let Some(identity) = parse_project_override_key(&record.key)? else {
            continue;
        };
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
        let Some(identity) = parse_project_override_key(&record.key)? else {
            continue;
        };
        if identity.project_id != project_id {
            continue;
        }
        let content = record.value.as_str().ok_or_else(|| {
            invalid_argument(format!("提示词覆盖记录 value 必须是字符串: {}", record.key))
        })?;
        super::identity::validate_project_override_content(content)?;
        if !active_prompt_names.contains(&identity.name) {
            continue;
        }
        overrides.push(ProjectPromptOverrideSummary {
            project_id,
            prompt_name: identity.name,
            content: record.value,
            updated_at: record.updated_at.to_string(),
        });
    }

    Ok(overrides)
}
