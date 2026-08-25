use super::{
    ConfirmReverseEngineeringCandidateInput, GetReverseEngineeringCandidateInput,
    ReverseEngineerInput, ReverseEngineeringCandidate, ReverseEngineeringTarget,
};
use crate::{AppState, actions::access};
use a4n_core::error::AppError;

pub struct AuthorizedReverseEngineerInput {
    input: ReverseEngineerInput,
}

impl AuthorizedReverseEngineerInput {
    pub fn project_id(&self) -> i64 {
        self.input.project_id
    }

    pub fn target(&self) -> ReverseEngineeringTarget {
        self.input.target
    }

    pub(super) fn into_input(self) -> ReverseEngineerInput {
        self.input
    }
}

pub async fn authorize_reverse_engineer(
    state: &AppState,
    user_id: i64,
    input: ReverseEngineerInput,
) -> Result<AuthorizedReverseEngineerInput, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    Ok(AuthorizedReverseEngineerInput { input })
}

pub async fn reverse_engineer(
    state: &AppState,
    authorized: AuthorizedReverseEngineerInput,
    progress: &(dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync),
    active_task_id: Option<&str>,
) -> Result<ReverseEngineeringCandidate, AppError> {
    let input = authorized.input;
    let project_id = input.project_id;
    let target = input.target;
    let prompt_renderer = state;
    let mut result = state
        .orchestrator()
        .run_reverse_engineering_validated(input, prompt_renderer, progress, active_task_id)
        .await?;
    if target == ReverseEngineeringTarget::Blueprint {
        result
            .warnings
            .extend(reverse_material_sync_warnings(state, project_id, target).await);
    }
    Ok(result)
}

pub async fn get_reverse_engineering_candidate(
    state: &AppState,
    user_id: i64,
    input: &GetReverseEngineeringCandidateInput,
) -> Result<Option<ReverseEngineeringCandidate>, AppError> {
    access::verify_project_access(state, user_id, input.project_id).await?;
    state
        .orchestrator()
        .latest_reverse_engineering_candidate_validated(input.project_id, input.target)
        .await
}

pub async fn confirm_reverse_engineering_candidate(
    state: &AppState,
    user_id: i64,
    input: &ConfirmReverseEngineeringCandidateInput,
) -> Result<ReverseEngineeringCandidate, AppError> {
    let run = state
        .orchestrator()
        .workflow_run_model_by_id(input.workflow_run_id)
        .await?;
    let project_id = run
        .project_id
        .filter(|project_id| *project_id > 0)
        .ok_or_else(|| AppError::InvalidState {
            current: format!(
                "workflow_run_id={}, project_id={:?}, root_project_id={:?}",
                run.id, run.project_id, run.root_project_id
            ),
            operation: "确认项目资料归纳候选".to_string(),
        })?;
    access::verify_project_editor(state, user_id, project_id).await?;
    let mut result = state
        .orchestrator()
        .confirm_reverse_engineering_candidate_validated(input.workflow_run_id)
        .await?;
    let target = result.target;
    result
        .warnings
        .extend(reverse_material_sync_warnings(state, project_id, target).await);
    Ok(result)
}

async fn reverse_material_sync_warnings(
    state: &AppState,
    project_id: i64,
    target: ReverseEngineeringTarget,
) -> Vec<String> {
    match target {
        ReverseEngineeringTarget::Blueprint => {
            crate::actions::blueprint_records::blueprint_material_sync_warnings(state, project_id)
                .await
        }
        ReverseEngineeringTarget::Outline | ReverseEngineeringTarget::PartOutline => {
            crate::actions::outline_records::outline_material_sync_warnings(state, project_id).await
        }
    }
}
