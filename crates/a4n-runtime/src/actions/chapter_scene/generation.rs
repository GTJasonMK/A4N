use super::access_errors::{required_scene_generation_inputs, verify_chapter_access};
use super::contracts::chapter_scene_summary;
use super::contracts::{
    ChapterSceneGenerationRequestContext, ChapterSceneProgressReporter, ChapterSceneSummary,
    ChapterSceneTaskScope, GenerateChapterSceneInput,
};
use super::output::normalize_scene_generation_output;
use super::request::build_scene_generation_request_for_input;
use super::workflow::{
    chapter_scene_business_key, chapter_scene_call_fingerprint_json,
    chapter_scene_workflow_fingerprint, finalize_chapter_scene_workflow,
    restored_chapter_scene_checkpoint,
};
use crate::actions::workflow_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    required_llm_call_checkpoint_json,
};
use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_scene;
use a4n_llm::provider::is_truncated_finish_reason;
use a4n_orchestrator::project::ProjectGeneratedChapterSceneSaveInput;
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use serde_json::{Value, json};

pub async fn generate_chapter_scene(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterSceneInput,
    progress: ChapterSceneProgressReporter<'_>,
) -> Result<ChapterSceneSummary, AppError> {
    progress(0, 4, "正在读取章节和场景参数")?;
    let context = build_scene_generation_request_for_input(state, user_id, &input).await?;

    progress(1, 4, "正在构造场景生成提示词")?;
    progress(2, 4, "正在调用模型生成场景正文")?;
    let workflow_fingerprint = chapter_scene_workflow_fingerprint(&input, &context.request)?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: chapter_scene_business_key(&workflow_fingerprint)?,
            workflow_kind: "chapter_scene_generation".to_string(),
            project_id: Some(context.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter".to_string()),
            scope_id: Some(input.chapter_id),
            chapter_id: Some(input.chapter_id),
            mode: Some("scene".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(context.call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let result = async {
        let content = run_chapter_scene_llm_call(
            state,
            workflow_run.id,
            workflow_fingerprint,
            context,
            progress,
        )
        .await?;
        progress(3, 4, "模型已返回，正在保存场景")?;
        let scene = save_generated_chapter_scene(state, &input, &content).await?;
        progress(4, 4, "场景已保存")?;
        Ok(chapter_scene_summary(&scene))
    }
    .await;
    finalize_chapter_scene_workflow(state, workflow_run.id, &result).await?;
    result
}

async fn run_chapter_scene_llm_call(
    state: &AppState,
    workflow_run_id: i64,
    workflow_fingerprint: Value,
    mut context: ChapterSceneGenerationRequestContext,
    progress: ChapterSceneProgressReporter<'_>,
) -> Result<String, AppError> {
    let call_key = context.call_key.clone();
    let fingerprint_json =
        chapter_scene_call_fingerprint_json(&call_key, &workflow_fingerprint, &context.request)?;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
    if let Some(trace_context) = context.request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await?;
    if let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &fingerprint_json)
        .await?
    {
        progress(3, 4, "场景正文已从断点恢复，正在保存场景")?;
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
        return restored_chapter_scene_checkpoint(checkpoint_json, &call_key);
    }

    let response = state
        .llm
        .chat(context.request)
        .await
        .map_err(|err| AppError::LlmError(err.to_string()))?;
    if is_truncated_finish_reason(response.finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(
            "场景正文生成输出达到当前 max_tokens 上限，正文可能被截断；请在设置页调大“标准”输出档位，或缩短本次场景目标后重试".to_string(),
        ));
    }
    let content = normalize_scene_generation_output(&response.message.content)?;
    let response_usage_json = response
        .usage
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    state
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": content.clone(),
                "parsed_output": content.clone(),
                "response_content": response.message.content,
                "finish_reason": response.finish_reason,
                "usage": response_usage_json}),
            artifacts_json: None,
        })
        .await?;
    Ok(content)
}

async fn save_generated_chapter_scene(
    state: &AppState,
    input: &GenerateChapterSceneInput,
    content: &str,
) -> Result<chapter_scene::Model, AppError> {
    state
        .orchestrator
        .save_generated_chapter_scene_record(ProjectGeneratedChapterSceneSaveInput {
            chapter_id: input.chapter_id,
            scene_number: input.scene_number,
            content: content.to_string(),
        })
        .await
}

pub async fn generate_chapter_scene_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ChapterSceneTaskScope, AppError> {
    let ch = verify_chapter_access(state, user_id, input.chapter_id).await?;
    required_scene_generation_inputs(&ch, input)?;
    Ok(ChapterSceneTaskScope {
        project_id: ch.project_id,
        total: 4,
    })
}
