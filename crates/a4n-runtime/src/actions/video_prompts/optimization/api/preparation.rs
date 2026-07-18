use super::super::super::{
    VideoPromptMutationPlan, prepare_video_prompt_mutation_plan,
    validate_project_chapter_video_source,
};
use super::super::contracts::{
    CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
    CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME, OptimizeChapterVideoPromptInput,
};
use super::super::request::build_video_prompt_optimization_request_for_state;
use crate::{
    AppState,
    actions::{
        access, app_error_message, workflow_checkpoint::llm_call_checkpoint_fingerprint_sha256,
    },
};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::prompt_language::prompt_output_language_from_instruction;
use a4n_llm::{provider::ChatRequest, trace::context_aware_chat_request_trace_json};
use a4n_orchestrator::{
    generation::generation_checkpoint_fingerprint_sha256,
    video::PromptOutputLanguage,
    workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING},
};
use serde_json::{Value, json};

pub(super) struct PreparedVideoPromptOptimization {
    pub(super) prompt: chapter_video_prompt::Model,
    pub(super) mutation_plan: VideoPromptMutationPlan,
    pub(super) prompt_output_language: PromptOutputLanguage,
    pub(super) request: ChatRequest,
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) generation_input_fingerprint_sha256: String,
    pub(super) input_fingerprint_json: Value,
    pub(super) messages_json: Value,
    pub(super) input_variables_json: Value,
    pub(super) source_refs_json: Value,
    pub(super) media_refs_json: Option<Value>,
    pub(super) request_audit_json: Value,
}

pub(super) async fn prepare_video_prompt_optimization(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    input: OptimizeChapterVideoPromptInput,
) -> Result<PreparedVideoPromptOptimization, String> {
    let instruction = input.instruction.trim();
    if instruction.is_empty() {
        return Err("优化要求不能为空".to_string());
    }
    let prompt = state
        .orchestrator
        .chapter_video_prompt(prompt_id)
        .await
        .map_err(app_error_message)?;
    access::verify_project_editor_string(state, user_id, prompt.project_id).await?;
    validate_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "优化视频提示词",
        true,
    )
    .await?;
    let mutation_plan = prepare_video_prompt_mutation_plan(
        state,
        prompt_id,
        input.downstream_strategy,
        "优化视频提示词",
    )
    .await?;

    let current_language = PromptOutputLanguage::from_metadata(prompt.metadata_json.as_ref());
    let prompt_output_language =
        prompt_output_language_from_instruction(instruction, current_language);
    let mut request = build_video_prompt_optimization_request_for_state(
        state,
        &prompt,
        instruction,
        prompt_output_language,
    )
    .await?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(&request.messages)
        .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let input_variables_json = json!({
        "instruction": instruction,
        "prompt_id": prompt.id,
        "prompt_type": &prompt.prompt_type,
        "workflow_mode": &prompt.workflow_mode,
        "shot_number": prompt.shot_number,
        "prompt_output_language": prompt_output_language.as_str()});
    let source_refs_json = json!({
        "previous_prompt_id": prompt.id,
        "previous_visual_prompt": &prompt.visual_prompt,
        "previous_motion_prompt": &prompt.motion_prompt,
        "previous_negative_prompt": &prompt.negative_prompt});
    let media_refs_json = prompt.reference_image_ids_json.clone();
    let input_fingerprint_json = json!({
        "prompt_names": [
            CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
            CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        ],
        "request": request_json,
        "input_variables": input_variables_json.clone(),
        "source_refs": source_refs_json.clone(),
        "media_refs": media_refs_json.clone()});
    let call_key = format!("optimize_chapter_video_prompt:prompt:{prompt_id}");
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "video_prompt_optimization:{}:{prompt_id}",
                prompt.project_id
            ),
            workflow_kind: "video_prompt_optimization".to_string(),
            project_id: Some(prompt.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter_video_prompt".to_string()),
            scope_id: Some(prompt_id),
            chapter_id: Some(prompt.chapter_id),
            mode: Some(prompt.workflow_mode.clone()),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;
    let generation_input_fingerprint_sha256 =
        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(|err| err.to_string())?;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    let request_audit_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;

    Ok(PreparedVideoPromptOptimization {
        prompt,
        mutation_plan,
        prompt_output_language,
        request,
        workflow_run_id,
        call_key,
        generation_input_fingerprint_sha256,
        input_fingerprint_json,
        messages_json,
        input_variables_json,
        source_refs_json,
        media_refs_json,
        request_audit_json,
    })
}
