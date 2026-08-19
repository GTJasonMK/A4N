use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};
use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use serde_json::{Value, json};

use crate::workflow::workflow_checkpoint_contract_app_error;
use crate::{OrchestratorContext, structured_llm::ensure_checkpoint_parsed_output_matches};

pub mod design_brief;

mod application;
mod generation;
mod llm_stream;
mod prompt_generation;
mod prompt_rules;
mod reference_id_rules;
mod workflow_rules;

pub use application::{
    DecodedPortraitImage, GenerateAgeVariantInput, GenerateMissingPortraitsInput,
    GeneratePortraitInput, GeneratePortraitPromptInput, GeneratedPortraitImage,
    PersistPortraitFileInput, PersistedPortraitFile, PortraitBatchItem, PortraitBatchOutcome,
    PortraitGenerationHostPort, PortraitImageProviderInput, PortraitProgress,
    PortraitReferenceImageSource, PortraitSummary, PortraitType, RegeneratePortraitInput,
    UploadPortraitInput, portrait_summary,
};
pub use prompt_generation::{
    PortraitPromptGenerationInput, PortraitPromptRequestInput, PortraitPromptSpec,
    build_generate_prompt_request, normalize_portrait_prompt_response,
};
pub use prompt_rules::{
    DEFAULT_PORTRAIT_EXPRESSION, expression_or_default, optional_profile_text,
    portrait_profile_context, portrait_style_with_pose, profile_portrait_seed_prompt,
};
pub use reference_id_rules::validate_reference_image_ids;
pub use workflow_rules::{
    missing_portrait_item_fingerprint, portrait_generate_business_key,
    portrait_generate_workflow_fingerprint, portrait_image_business_key,
    portrait_image_workflow_fingerprint, portrait_prompt_business_key,
    portrait_prompt_checkpoint_fingerprint, portrait_prompt_workflow_fingerprint,
};

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use a4n_core::models::generation::workflow_run;

#[cfg(test)]
mod tests;

impl OrchestratorContext {
    /// 完整立绘生成：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_portrait_generation_workflow_validated(
        &self,
        project_id: i64,
        profile_id: i64,
        fingerprint: &Value,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if profile_id <= 0 {
            return Err(AppError::InvalidArgument(
                "角色档案 ID 必须大于 0".to_string(),
            ));
        }
        let business_key =
            portrait_generate_business_key(fingerprint).map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(UpsertWorkflowRunInput {
            business_key,
            workflow_kind: "portrait_generation".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(profile_id),
            chapter_id: None,
            mode: Some("full_body".to_string()),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_generation.prompt".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }

    /// 人物三视图提示词生成：校验后创建 workflow_run。
    pub(crate) async fn begin_portrait_prompt_workflow_validated(
        &self,
        project_id: i64,
        profile_id: i64,
        portrait_type: PortraitType,
        fingerprint: &Value,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if profile_id <= 0 {
            return Err(AppError::InvalidArgument(
                "角色档案 ID 必须大于 0".to_string(),
            ));
        }
        let business_key =
            portrait_prompt_business_key(fingerprint).map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(UpsertWorkflowRunInput {
            business_key,
            workflow_kind: "portrait_prompt_generation".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(profile_id),
            chapter_id: None,
            mode: Some(portrait_type.to_string()),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_prompt.generate".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }

    /// 缺失立绘批量生成：校验后创建 workflow_run。
    pub(crate) async fn begin_missing_portrait_workflow_validated(
        &self,
        project_id: i64,
        fingerprint: &Value,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        self.begin_workflow_run_validated(UpsertWorkflowRunInput {
            business_key: format!("missing_portrait_generation:{project_id}"),
            workflow_kind: "missing_portrait_generation".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some("missing".to_string()),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("missing_portrait_generation.items".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }

    /// 立绘图片类工作流（再生成 / 年龄变体）：校验后创建 workflow_run。
    pub(crate) async fn begin_portrait_image_workflow_validated(
        &self,
        workflow_kind: &str,
        project_id: i64,
        source_portrait_id: i64,
        mode: &str,
        current_step_key: &str,
        fingerprint: &Value,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if source_portrait_id <= 0 {
            return Err(AppError::InvalidArgument("立绘 ID 必须大于 0".to_string()));
        }
        if workflow_kind.trim().is_empty() || current_step_key.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "workflow_kind / current_step_key 不能为空".to_string(),
            ));
        }
        let business_key = portrait_image_business_key(workflow_kind, fingerprint)
            .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(UpsertWorkflowRunInput {
            business_key,
            workflow_kind: workflow_kind.to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("character_portrait".to_string()),
            scope_id: Some(source_portrait_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(current_step_key.to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }

    pub(crate) async fn build_generate_prompt_request(
        &self,
        input: PortraitPromptRequestInput<'_>,
    ) -> Result<ChatRequest, AppError> {
        build_generate_prompt_request(input).await
    }

    pub(crate) async fn generate_prompt_with_checkpoint(
        &self,
        input: PortraitPromptGenerationInput<'_>,
    ) -> Result<String, AppError> {
        let PortraitPromptGenerationInput {
            project_id,
            spec,
            prompt_renderer,
            variables,
            workflow_run_id,
        } = input;
        let mut request = build_generate_prompt_request(PortraitPromptRequestInput {
            project_id,
            spec,
            prompt_renderer,
            variables,
        })
        .await?;

        let call_key = "portrait_prompt.generate".to_string();
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let fingerprint_json = portrait_prompt_call_fingerprint_json(
            project_id,
            spec.profile_name,
            spec.portrait_type,
            &request,
        )?;
        crate::workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )?;

        self.mark_workflow_current_step(workflow_run_id, &call_key)
            .await?;
        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &fingerprint_json)
            .await?
        {
            return restored_portrait_prompt_checkpoint(
                &checkpoint.parsed_output,
                &checkpoint.response_content,
                &call_key,
            );
        }

        let response = llm_stream::execute_portrait_llm_call(self, request, &call_key).await?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "人物三视图提示词输出达到当前 max_tokens 上限，提示词可能被截断；请在设置页调大“媒体提示词”输出档位，或减少角色上下文后重试".to_string(),
            ));
        }
        let output = normalize_portrait_prompt_response(&response.message.content)?;
        let usage = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: Value::String(output.clone()),
                response_content: response.message.content,
                usage,
                finish_reason: response.finish_reason,
                artifacts_json: None,
            },
        })
        .await?;
        Ok(output)
    }
}

fn portrait_prompt_call_fingerprint_json(
    project_id: i64,
    profile_name: &str,
    portrait_type: PortraitType,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "portrait_prompt_call_v1",
        "workflow": "portrait_prompt_generation",
        "call_key": "portrait_prompt.generate",
        "project_id": project_id,
        "profile_name": profile_name,
        "portrait_type": portrait_type,
        "request": a4n_llm::trace::context_aware_chat_request_trace_json(request)?,
    }))
}

fn restored_portrait_prompt_checkpoint(
    parsed_output: &Value,
    response_content: &str,
    call_key: &str,
) -> Result<String, AppError> {
    let parsed_output = parsed_output
        .as_str()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_checkpoint_field:parsed_output".to_string(),
            operation: format!("读取人物三视图提示词 LLM 调用断点 {call_key}"),
        })?;
    let reparsed = normalize_portrait_prompt_response(response_content)?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &reparsed,
        &format!("人物三视图提示词 {call_key}"),
    )?;
    Ok(reparsed)
}
