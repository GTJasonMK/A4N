use super::super::super::{VideoPromptMutationPlan, apply_video_prompt_downstream_deletions};
use super::super::contracts::{
    CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
    CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
};
use super::call::VideoPromptOptimizationCallResult;
use crate::{AppState, actions::app_error_message};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::prompt_language::with_prompt_output_language_metadata;
use a4n_orchestrator::{
    generation::{
        GenerationRunCheckpointContext, RecordCompletedGenerationRunInput,
        RecordCompletedGenerationRunWithCheckpointInput,
    },
    video::{PromptOutputLanguage, UpdateChapterVideoPromptInput},
};
use serde_json::{Value, json};

pub(super) struct VideoPromptOptimizationPersistenceInput {
    pub(super) prompt: chapter_video_prompt::Model,
    pub(super) mutation_plan: VideoPromptMutationPlan,
    pub(super) prompt_output_language: PromptOutputLanguage,
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

pub(super) async fn persist_video_prompt_optimization_result(
    state: &AppState,
    user_id: i64,
    input: VideoPromptOptimizationPersistenceInput,
    call_result: VideoPromptOptimizationCallResult,
) -> Result<chapter_video_prompt::Model, String> {
    let VideoPromptOptimizationPersistenceInput {
        prompt,
        mutation_plan,
        prompt_output_language,
        workflow_run_id,
        call_key,
        generation_input_fingerprint_sha256,
        input_fingerprint_json,
        messages_json,
        input_variables_json,
        source_refs_json,
        media_refs_json,
        request_audit_json,
    } = input;
    let VideoPromptOptimizationCallResult {
        response_text,
        response_usage_json,
        response_finish_reason,
        visual_prompt,
        motion_prompt,
        negative_prompt,
    } = call_result;
    let optimized_metadata = with_prompt_output_language_metadata(
        prompt.metadata_json.clone().unwrap_or_else(|| json!({})),
        prompt_output_language,
    );

    apply_video_prompt_downstream_deletions(state, user_id, &mutation_plan).await?;
    state
        .orchestrator
        .update_chapter_video_prompt_with_generation_run_checkpoint(
            prompt.id,
            UpdateChapterVideoPromptInput {
                visual_prompt: Some(visual_prompt.clone()),
                motion_prompt: Some(motion_prompt.clone()),
                negative_prompt: Some(negative_prompt.clone()),
                duration_secs: None,
                reference_image_ids_json: None,
            },
            Some(optimized_metadata),
            mutation_plan.allow_downstream,
            RecordCompletedGenerationRunWithCheckpointInput {
                run: RecordCompletedGenerationRunInput {
                    project_id: prompt.project_id,
                    workflow: "video_prompt_generation".to_string(),
                    step_key: "optimize_chapter_video_prompt".to_string(),
                    title: "视频提示词优化".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!([
                        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
                        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
                    ])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(messages_json),
                    rendered_prompts_json: None,
                    input_variables_json: Some(input_variables_json),
                    source_refs_json: Some(source_refs_json),
                    media_refs_json,
                    request_json: Some(request_audit_json),
                    response_summary_json: Some(json!({
                        "visual_prompt_chars": visual_prompt.chars().count(),
                        "motion_prompt_chars": motion_prompt.as_deref().map(|value| value.chars().count()),
                        "negative_prompt_chars": negative_prompt.as_deref().map(|value| value.chars().count())})),
                    targets: Vec::new(),
                },
                checkpoint: GenerationRunCheckpointContext {
                    workflow_run_id: Some(workflow_run_id),
                    call_key: Some(call_key),
                    input_fingerprint_sha256: Some(generation_input_fingerprint_sha256),
                    input_fingerprint_json: Some(input_fingerprint_json),
                    raw_response_json: Some(json!({
                        "text": response_text,
                        "usage": response_usage_json,
                        "finish_reason": response_finish_reason})),
                    parsed_output_json: Some(json!({
                        "optimized_fields": {
                            "visual_prompt": visual_prompt,
                            "motion_prompt": motion_prompt,
                            "negative_prompt": negative_prompt}})),
                    validation_json: None,
                    artifact_refs_json: None,
                    attempt: None,
                },
            },
        )
        .await
        .map_err(app_error_message)
}
