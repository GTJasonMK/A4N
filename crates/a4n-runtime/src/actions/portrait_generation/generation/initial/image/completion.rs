use a4n_core::models::character::{character_portrait, protagonist_profile};
use a4n_orchestrator::{
    generation::{
        GenerationRunCheckpointContext, GenerationRunTraceTarget,
        RecordCompletedGenerationRunInput, RecordCompletedGenerationRunWithCheckpointInput,
        generation_checkpoint_fingerprint_sha256,
    },
    workflow::CompletedWorkflowCheckpointInput,
};
use serde_json::{Value, json};

use crate::AppState;

use super::super::super::super::{GeneratePortraitInput, image_pipeline};
use super::super::prompt::InitialPortraitPrompt;

pub(super) struct InitialPortraitCompletionInput<'a> {
    pub(super) input: &'a GeneratePortraitInput,
    pub(super) profile: &'a protagonist_profile::Model,
    pub(super) expression: &'a str,
    pub(super) workflow_run_id: i64,
    pub(super) workflow_fingerprint: Value,
    pub(super) prompt: InitialPortraitPrompt,
}

pub(super) async fn complete_initial_portrait(
    state: &AppState,
    portrait: &character_portrait::Model,
    completion: InitialPortraitCompletionInput<'_>,
) -> Result<(), String> {
    let InitialPortraitCompletionInput {
        input,
        profile,
        expression,
        workflow_run_id,
        workflow_fingerprint,
        prompt,
    } = completion;
    let request_json = serde_json::to_value(&prompt.request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(&prompt.request.messages)
        .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let input_variables_json = json!({
        "profile_id": profile.id,
        "character_name": &profile.name,
        "portrait_type": "full_body",
        "expression": expression,
        "pose": &input.pose,
        "style": &input.style,
        "reference_image_ids": &input.reference_image_ids});
    let source_refs_json = json!({
        "profile_id": profile.id,
        "appearance": &profile.appearance,
        "gender": &profile.gender,
        "age": &profile.age,
        "personality": &profile.personality,
        "background": &profile.background});
    let media_refs_json = json!({
        "reference_image_ids": &input.reference_image_ids});
    let input_fingerprint_json = json!({
        "prompt_name": "character_portrait_generation",
        "project_id": input.project_id,
        "request": request_json.clone(),
        "input_variables": input_variables_json.clone(),
        "source_refs": source_refs_json.clone(),
        "media_refs": media_refs_json.clone()});

    if let Err(err) = state
        .orchestrator
        .record_completed_generation_run_with_checkpoint(
            RecordCompletedGenerationRunWithCheckpointInput {
                run: RecordCompletedGenerationRunInput {
                    project_id: input.project_id,
                    workflow: "portrait_generation".to_string(),
                    step_key: "generate_portrait_prompt".to_string(),
                    title: "人物立绘提示词生成".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!(["character_portrait_generation"])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(messages_json),
                    rendered_prompts_json: None,
                    input_variables_json: Some(input_variables_json),
                    source_refs_json: Some(source_refs_json),
                    media_refs_json: Some(media_refs_json),
                    request_json: Some(request_json),
                    response_summary_json: Some(json!({
                        "character_portrait_id": portrait.id,
                        "prompt_chars": prompt.image_prompt.chars().count(),
                        "image_path": &portrait.image_path})),
                    targets: vec![GenerationRunTraceTarget {
                        target_kind: "character_portrait".to_string(),
                        target_table: "character_portrait".to_string(),
                        target_id: portrait.id,
                    }],
                },
                checkpoint: GenerationRunCheckpointContext {
                    workflow_run_id: Some(workflow_run_id),
                    call_key: Some(format!(
                        "character_portrait_generation:profile:{}:full_body",
                        profile.id
                    )),
                    input_fingerprint_sha256: Some(
                        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                            .map_err(|err| err.to_string())?,
                    ),
                    input_fingerprint_json: Some(input_fingerprint_json),
                    raw_response_json: Some(prompt.raw_response_json),
                    parsed_output_json: Some(json!({
                        "character_portrait_id": portrait.id,
                        "profile_id": profile.id,
                        "portrait_type": "full_body",
                        "image_prompt": prompt.image_prompt,
                        "image_path": &portrait.image_path})),
                    validation_json: None,
                    artifact_refs_json: Some(json!({
                        "targets": [{
                            "target_kind": "character_portrait",
                            "target_table": "character_portrait",
                            "target_id": portrait.id}]})),
                    attempt: None,
                },
            },
        )
        .await
    {
        return match image_pipeline::rollback_failed_portrait_creation(
            state,
            portrait.id,
            &portrait.image_path,
        )
        .await
        {
            Ok(()) => Err(err.to_string()),
            Err(cleanup_err) => Err(format!("{err}；{cleanup_err}")),
        };
    }

    state
        .orchestrator
        .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow_run_id),
            step_key: "portrait_generation.final".to_string(),
            step_order: 2,
            input_fingerprint_json: workflow_fingerprint,
            checkpoint_json: json!({
                "portrait_id": portrait.id,
                "image_path": portrait.image_path,
                "summary": image_pipeline::to_summary(portrait)}),
        })
        .await
        .map_err(|error| error.to_string())
}
