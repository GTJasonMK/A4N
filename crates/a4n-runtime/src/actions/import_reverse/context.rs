use super::contracts::ReverseEngineerInput;
use super::target::ReverseEngineeringTarget;
use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::import_reverse::{ReverseContentRequest, ReverseContextKind};
use a4n_orchestrator::import_reverse::resolve_reverse_content_sample;
use serde_json::Value;

pub(super) async fn apply_reverse_engineering_result(
    state: &AppState,
    project_id: i64,
    target: ReverseEngineeringTarget,
    result: &Value,
) -> Result<Value, AppError> {
    let applied = match target {
        ReverseEngineeringTarget::Blueprint => {
            state
                .apply_reverse_blueprint_data(project_id, result)
                .await?
        }
        ReverseEngineeringTarget::Outline => {
            state.apply_reverse_outline_data(project_id, result).await?
        }
        ReverseEngineeringTarget::PartOutline => {
            state
                .apply_reverse_part_outline_data(project_id, result)
                .await?
        }
    };
    state
        .orchestrator
        .advance_project_status_if_allowed(project_id, target.applied_status())
        .await?;
    Ok(applied)
}

pub(super) async fn reverse_content_sample(
    state: &AppState,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
) -> Result<String, AppError> {
    let rows = if input.auto_context && input.context_limit.is_some_and(|limit| limit > 0) {
        Some(
            state
                .orchestrator
                .reverse_context_rows(
                    input.project_id,
                    matches!(target.context_kind(), ReverseContextKind::Blueprint),
                )
                .await?,
        )
    } else {
        None
    };
    resolve_reverse_content_sample(
        ReverseContentRequest {
            project_id: input.project_id,
            manual_sample: &input.content_sample,
            auto_context: input.auto_context,
            context_limit: input.context_limit,
        },
        target.context_kind(),
        rows.as_ref(),
    )
    .map_err(AppError::InvalidArgument)
}
