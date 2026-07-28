use a4n_core::error::AppError;
use apply::{
    reverse_blueprint_apply_draft, reverse_blueprint_apply_result, reverse_outline_apply_result,
    reverse_outline_rows, reverse_part_outline_apply_result, reverse_part_outline_rows,
};
use serde_json::Value;
use std::collections::HashMap;

use crate::OrchestratorContext;
use crate::workflow::WorkflowRunFinalizeOutcome;

use super::request::{ReverseEngineerRequestInput, build_reverse_engineer_request};
use super::{ReverseEngineeringTarget, resolve_reverse_content_sample};
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
    pub apply: bool,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub progress: &'a ReverseProgressCallback<'a>,
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
    pub async fn reverse_apply_blueprint_from_context_validated(
        &self,
        project_id: i64,
        context_limit: usize,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Value, AppError> {
        let target = ReverseEngineeringTarget::Blueprint;
        let variables = HashMap::new();
        let content_sample = self
            .resolve_reverse_content_sample_validated(
                project_id,
                "",
                true,
                Some(context_limit),
                target,
            )
            .await?;
        let request = build_reverse_engineer_request(ReverseEngineerRequestInput {
            project_id,
            prompt_name: target.prompt_name(),
            user_prefix: target.user_instruction(),
            content_sample: &content_sample,
            prompt_renderer,
            variables: &variables,
            max_tokens_profile: target.max_tokens_profile(),
        })
        .await?;
        let result = self
            .reverse_engineer_request(target.prompt_name(), request, None)
            .await?;
        self.apply_reverse_blueprint_data(project_id, &result).await
    }

    pub async fn prepare_reverse_engineer_request_validated(
        &self,
        input: ReverseEngineerPreviewInput<'_>,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        let content_sample = self
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
        let total = if input.apply { 4 } else { 3 };
        (input.progress)(0, total, input.target.preparing_context_message())?;
        let content_sample = self
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
                input.apply,
                &content_sample,
                input.auto_context,
                context_limit,
                input.variables,
            )
            .await?;
        let result = self
            .run_reverse_engineering_workflow(input, total, workflow.id, &content_sample)
            .await;
        let outcome = match &result {
            Ok(_) => WorkflowRunFinalizeOutcome::Completed,
            Err(AppError::Cancelled) => WorkflowRunFinalizeOutcome::Cancelled,
            Err(_) => WorkflowRunFinalizeOutcome::Failed,
        };
        let error_message = result.as_ref().err().map(ToString::to_string);
        let finalized = self
            .finalize_workflow_run_outcome_validated(workflow.id, outcome, error_message.as_deref())
            .await;
        match (result, finalized) {
            (Ok(value), Ok(_)) => Ok(value),
            (Ok(_), Err(status_error)) => Err(status_error),
            (Err(error), Ok(_)) => Err(error),
            (Err(error), Err(status_error)) => Err(AppError::InvalidState {
                current: error.to_string(),
                operation: format!("导入反推失败，且 workflow 终态写入失败: {status_error}"),
            }),
        }
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
        if !input.apply {
            return Ok(result);
        }
        (input.progress)(3, total, input.target.applying_message())?;
        let applied = self
            .apply_reverse_engineering_result(input.project_id, input.target, &result)
            .await?;
        (input.progress)(4, total, input.target.applied_message())?;
        Ok(serde_json::json!({
            "result": result,
            "applied": applied,
        }))
    }

    async fn resolve_reverse_content_sample_validated(
        &self,
        project_id: i64,
        manual_sample: &str,
        auto_context: bool,
        context_limit: Option<usize>,
        target: ReverseEngineeringTarget,
    ) -> Result<String, AppError> {
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
        resolve_reverse_content_sample(
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

    async fn apply_reverse_engineering_result(
        &self,
        project_id: i64,
        target: ReverseEngineeringTarget,
        data: &Value,
    ) -> Result<Value, AppError> {
        let applied = match target {
            ReverseEngineeringTarget::Blueprint => {
                self.apply_reverse_blueprint_data(project_id, data).await?
            }
            ReverseEngineeringTarget::Outline => {
                self.apply_reverse_outline_data(project_id, data).await?
            }
            ReverseEngineeringTarget::PartOutline => {
                self.apply_reverse_part_outline_data(project_id, data)
                    .await?
            }
        };
        self.advance_project_status_if_allowed(project_id, target.applied_status())
            .await?;
        Ok(applied)
    }

    pub(crate) async fn apply_reverse_blueprint_data(
        &self,
        project_id: i64,
        data: &Value,
    ) -> Result<Value, AppError> {
        let draft = reverse_blueprint_apply_draft(data).map_err(AppError::InvalidArgument)?;
        let result = self
            .apply_reverse_blueprint_records_transaction(project_id, draft)
            .await?;
        Ok(reverse_blueprint_apply_result(result))
    }

    pub(crate) async fn apply_reverse_outline_data(
        &self,
        project_id: i64,
        data: &Value,
    ) -> Result<Value, AppError> {
        let rows = reverse_outline_rows(data).map_err(AppError::InvalidArgument)?;
        let count = self
            .apply_reverse_outline_records_transaction(project_id, &rows)
            .await?;
        Ok(reverse_outline_apply_result(count))
    }

    pub(crate) async fn apply_reverse_part_outline_data(
        &self,
        project_id: i64,
        data: &Value,
    ) -> Result<Value, AppError> {
        let rows = reverse_part_outline_rows(data).map_err(AppError::InvalidArgument)?;
        let count = self
            .apply_reverse_part_outline_records_transaction(project_id, &rows)
            .await?;
        Ok(reverse_part_outline_apply_result(count))
    }
}
