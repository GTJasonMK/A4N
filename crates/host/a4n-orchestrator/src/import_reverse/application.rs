use a4n_api_contract::import_reverse::{
    REVERSE_ENGINEERING_PROGRESS_TOTAL, ReverseEngineerInput, ReverseEngineeringCandidate,
};
use a4n_error::AppError;
pub(in crate::import_reverse) use apply::{
    reverse_blueprint_apply_draft, reverse_blueprint_apply_result, reverse_outline_apply_result,
    reverse_outline_rows, reverse_part_outline_apply_result, reverse_part_outline_rows,
};
use serde_json::Value;

use crate::OrchestratorContext;

use super::ReverseEngineeringTarget;
use super::context::{ReverseContentRequest, ReverseContextKind};
use super::request::{ReverseEngineerRequestInput, build_reverse_engineer_request};
use super::target::ReverseEngineeringTargetDomain;
use a4n_prompt::rendering::PromptRenderer;

mod apply;
mod reverse_json;

pub(in crate::import_reverse) fn validate_reverse_engineer_output(
    target: ReverseEngineeringTarget,
    data: &Value,
) -> Result<(), AppError> {
    match target {
        ReverseEngineeringTarget::Blueprint => reverse_blueprint_apply_draft(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        ReverseEngineeringTarget::Outline => reverse_outline_rows(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        ReverseEngineeringTarget::PartOutline => reverse_part_outline_rows(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
    }
}

impl OrchestratorContext {
    pub async fn prepare_reverse_engineer_request_validated(
        &self,
        input: ReverseEngineerInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm_provider_contract::provider::ChatRequest, AppError> {
        let ReverseEngineerInput {
            project_id,
            content_sample: manual_sample,
            auto_context,
            context_limit,
            variables,
            target,
        } = input;
        let variables = variables.unwrap_or_default();
        let (content_sample, _) = self
            .resolve_reverse_content_sample_validated(
                project_id,
                &manual_sample,
                auto_context,
                context_limit,
                target,
            )
            .await?;
        build_reverse_engineer_request(ReverseEngineerRequestInput {
            project_id,
            target,
            content_sample: &content_sample,
            prompt_renderer,
            variables: &variables,
        })
        .await
    }

    pub async fn run_reverse_engineering_validated(
        &self,
        input: ReverseEngineerInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: &(dyn for<'a> Fn(u32, u32, &'a str) -> Result<(), AppError> + Send + Sync),
        active_task_id: Option<&str>,
    ) -> Result<ReverseEngineeringCandidate, AppError> {
        let ReverseEngineerInput {
            project_id,
            content_sample: manual_sample,
            auto_context,
            context_limit,
            variables,
            target,
        } = input;
        let variables = variables.unwrap_or_default();
        let total = REVERSE_ENGINEERING_PROGRESS_TOTAL;
        progress(0, total, target.preparing_context_message())?;
        let (content_sample, auto_context_source_sha256) = self
            .resolve_reverse_content_sample_validated(
                project_id,
                &manual_sample,
                auto_context,
                context_limit,
                target,
            )
            .await?;
        let context_limit = context_limit
            .map(i64::try_from)
            .transpose()
            .map_err(|_| AppError::InvalidArgument("context_limit 超出 i64 范围".to_string()))?;
        let workflow_input = super::workflow_rules::import_reverse_workflow_run_input(
            project_id,
            target,
            &content_sample,
            auto_context,
            context_limit,
            auto_context_source_sha256.as_deref(),
            &variables,
            active_task_id,
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow = self.begin_workflow_run_validated(workflow_input).await?;
        let result = async {
            progress(1, total, target.calling_model_message())?;
            let request = build_reverse_engineer_request(ReverseEngineerRequestInput {
                project_id,
                target,
                content_sample: &content_sample,
                prompt_renderer,
                variables: &variables,
            })
            .await?;
            let result = self
                .reverse_engineer_request(target, request, Some(workflow.id))
                .await?;
            progress(2, total, target.organizing_message())?;
            let result = if target == ReverseEngineeringTarget::Blueprint {
                super::candidate::apply_reverse_blueprint_workflow_result(self, workflow.id).await?
            } else {
                result
            };
            progress(3, total, target.ready_message())?;
            Ok(result)
        }
        .await;
        self.finalize_workflow_result(workflow.id, result, "导入反推")
            .await
    }

    async fn resolve_reverse_content_sample_validated(
        &self,
        project_id: i64,
        manual_sample: &str,
        auto_context: bool,
        context_limit: Option<usize>,
        target: ReverseEngineeringTarget,
    ) -> Result<(String, Option<String>), AppError> {
        let rows = if auto_context && context_limit.is_some_and(|limit| limit > 0) {
            Some(
                self.reverse_context_rows(
                    project_id,
                    matches!(target.context_kind(), ReverseContextKind::Blueprint),
                )
                .await?,
            )
        } else {
            None
        };
        super::context::resolve_reverse_content_sample_with_provenance(
            ReverseContentRequest {
                project_id,
                manual_sample,
                auto_context,
                context_limit,
            },
            target.context_kind(),
            rows.as_ref(),
        )
        .map_err(AppError::InvalidArgument)
    }
}
