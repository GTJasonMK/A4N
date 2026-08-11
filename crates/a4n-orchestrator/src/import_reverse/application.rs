use a4n_core::error::AppError;
pub(in crate::import_reverse) use apply::{
    reverse_blueprint_apply_draft, reverse_blueprint_apply_result, reverse_outline_apply_result,
    reverse_outline_rows, reverse_part_outline_apply_result, reverse_part_outline_rows,
};
use serde_json::Value;
use std::collections::HashMap;

use crate::OrchestratorContext;

use super::ReverseEngineeringTarget;
use super::request::{ReverseEngineerRequestInput, build_reverse_engineer_request};
use a4n_core::import_reverse::ReverseContentRequest;
use a4n_prompt::rendering::PromptRenderer;

mod apply;
mod reverse_json;

pub type ReverseProgressCallback<'a> =
    dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'a;

pub struct ReverseEngineerApplicationInput<'a> {
    pub project_id: i64,
    pub manual_sample: &'a str,
    pub auto_context: bool,
    pub context_limit: Option<usize>,
    pub variables: &'a HashMap<String, Value>,
    pub target: ReverseEngineeringTarget,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub progress: &'a ReverseProgressCallback<'a>,
    pub active_task_id: Option<&'a str>,
}

pub struct ReverseEngineerPreviewInput<'a> {
    pub project_id: i64,
    pub manual_sample: &'a str,
    pub auto_context: bool,
    pub context_limit: Option<usize>,
    pub variables: &'a HashMap<String, Value>,
    pub target: ReverseEngineeringTarget,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

pub(in crate::import_reverse) fn validate_reverse_engineer_output(
    prompt_name: &str,
    data: &Value,
) -> Result<(), AppError> {
    match prompt_name {
        "reverse_blueprint" => reverse_blueprint_apply_draft(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        "reverse_outline" => reverse_outline_rows(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        "reverse_part_outline" => reverse_part_outline_rows(data)
            .map(|_| ())
            .map_err(AppError::InvalidArgument),
        other => Err(AppError::InvalidArgument(format!(
            "未注册的导入反推结构化输出合同: {other}"
        ))),
    }
}

impl OrchestratorContext {
    pub async fn prepare_reverse_engineer_request_validated(
        &self,
        input: ReverseEngineerPreviewInput<'_>,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        let (content_sample, _) = self
            .resolve_reverse_content_sample_validated(
                input.project_id,
                input.manual_sample,
                input.auto_context,
                input.context_limit,
                input.target,
            )
            .await?;
        build_reverse_engineer_request(ReverseEngineerRequestInput {
            project_id: input.project_id,
            prompt_name: input.target.prompt_name(),
            user_prefix: input.target.user_instruction(),
            content_sample: &content_sample,
            prompt_renderer: input.prompt_renderer,
            variables: input.variables,
            max_tokens_profile: input.target.max_tokens_profile(),
        })
        .await
    }

    pub async fn run_reverse_engineering_validated(
        &self,
        input: ReverseEngineerApplicationInput<'_>,
    ) -> Result<Value, AppError> {
        let total = 3;
        (input.progress)(0, total, input.target.preparing_context_message())?;
        let (content_sample, auto_context_source_sha256) = self
            .resolve_reverse_content_sample_validated(
                input.project_id,
                input.manual_sample,
                input.auto_context,
                input.context_limit,
                input.target,
            )
            .await?;
        let context_limit = input
            .context_limit
            .map(i64::try_from)
            .transpose()
            .map_err(|_| AppError::InvalidArgument("context_limit 超出 i64 范围".to_string()))?;
        let workflow = self
            .begin_import_reverse_workflow_validated(
                input.project_id,
                input.target.prompt_name(),
                &content_sample,
                input.auto_context,
                context_limit,
                auto_context_source_sha256.as_deref(),
                input.variables,
                input.active_task_id,
            )
            .await?;
        let result = self
            .run_reverse_engineering_workflow(input, total, workflow.id, &content_sample)
            .await;
        self.finalize_workflow_outcome_result(workflow.id, result, "导入反推")
            .await
    }

    async fn run_reverse_engineering_workflow(
        &self,
        input: ReverseEngineerApplicationInput<'_>,
        total: u32,
        workflow_run_id: i64,
        content_sample: &str,
    ) -> Result<Value, AppError> {
        (input.progress)(1, total, input.target.calling_model_message())?;
        let request = build_reverse_engineer_request(ReverseEngineerRequestInput {
            project_id: input.project_id,
            prompt_name: input.target.prompt_name(),
            user_prefix: input.target.user_instruction(),
            content_sample,
            prompt_renderer: input.prompt_renderer,
            variables: input.variables,
            max_tokens_profile: input.target.max_tokens_profile(),
        })
        .await?;
        let result = self
            .reverse_engineer_request(input.target.prompt_name(), request, Some(workflow_run_id))
            .await?;
        (input.progress)(2, total, input.target.organizing_message())?;
        let result = if input.target == ReverseEngineeringTarget::Blueprint {
            super::candidate::apply_reverse_blueprint_workflow_result(self, workflow_run_id).await?
        } else {
            result
        };
        (input.progress)(3, total, input.target.ready_message())?;
        Ok(result)
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
                    matches!(
                        target.context_kind(),
                        a4n_core::import_reverse::ReverseContextKind::Blueprint
                    ),
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
