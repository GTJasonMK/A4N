use a4n_core::error::AppError;
use a4n_core::event::bridge::{EventBus, InternalEvent};
use a4n_core::models::project::blueprint_version;

use crate::AppState;
use crate::actions::access;

use super::contracts::{
    BlueprintDiffSummary, BlueprintSummary, BlueprintVersionSummary, DiffBlueprintVersionsInput,
    RestoreBlueprintVersionInput,
};
use super::diffing::json_diff;
use super::projections::{blueprint_summary, blueprint_version_summary};

pub async fn list_blueprint_versions(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<BlueprintVersionSummary>, AppError> {
    access::verify_project_access(state, user_id, project_id).await?;
    let versions = state
        .orchestrator
        .blueprint_versions_by_project(project_id)
        .await?;
    Ok(versions.iter().map(blueprint_version_summary).collect())
}

pub async fn diff_blueprint_versions(
    state: &AppState,
    user_id: i64,
    input: DiffBlueprintVersionsInput,
) -> Result<BlueprintDiffSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let version_a = load_blueprint_version(state, input.project_id, &input.version_a).await?;
    let version_b = load_blueprint_version(state, input.project_id, &input.version_b).await?;
    Ok(BlueprintDiffSummary {
        version_a: input.version_a,
        version_b: input.version_b,
        diff: json_diff(&version_a.blueprint_data, &version_b.blueprint_data),
    })
}

pub async fn restore_blueprint_version(
    state: &AppState,
    user_id: i64,
    input: RestoreBlueprintVersionInput,
) -> Result<BlueprintSummary, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let version_name = input.version.trim();
    if version_name.is_empty() {
        return Err(AppError::InvalidArgument("版本号不能为空".to_string()));
    }

    let result = state
        .orchestrator
        .restore_blueprint_version_record(input.project_id, version_name)
        .await?;

    if let Some(status_change) = result.status_change {
        state
            .event_bus
            .publish(InternalEvent::ProjectStatusChanged {
                project_id: input.project_id,
                from: status_change.from,
                to: status_change.to,
                timestamp: status_change.changed_at,
            })
            .await?;
    }
    state
        .event_bus
        .publish(InternalEvent::BlueprintUpdated {
            project_id: input.project_id,
            version: result.version,
        })
        .await?;
    Ok(blueprint_summary(&result.blueprint))
}

async fn load_blueprint_version(
    state: &AppState,
    project_id: i64,
    version: &str,
) -> Result<blueprint_version::Model, AppError> {
    state
        .orchestrator
        .blueprint_version_by_project_version(project_id, version)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("版本 {version} 不存在")))
}
