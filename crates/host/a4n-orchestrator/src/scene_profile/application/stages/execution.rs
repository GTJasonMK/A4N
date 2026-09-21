mod call;
mod checkpoint;

use serde_json::{Value, json};

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;

use crate::{
    OrchestratorContext,
    structured_llm::ensure_checkpoint_parsed_output_matches,
    workflow::{llm_call_checkpoint_fingerprint_sha256, workflow_checkpoint_contract_app_error},
};

use self::call::chat_scene_profile_stage;
use self::checkpoint::{
    load_completed_scene_profile_stage_checkpoint, save_completed_scene_profile_stage_checkpoint,
};
use super::super::parsing::parse_scene_stage_root_array;
use super::super::progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress};
use super::super::types::{SceneProfileStageProgressSpec, SceneProfileSyncRequestContext};
use super::request::scene_profile_stage_fingerprint;

pub(super) async fn run_scene_profile_stage_with_checkpoint(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    mut request: ChatRequest,
    context: &SceneProfileSyncRequestContext,
    prompt_name: &str,
    root_field: &str,
    step_key: String,
    step_order: i32,
    stage_progress: SceneProfileStageProgressSpec,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    response_chars: &mut Vec<Value>,
) -> Result<Value, AppError> {
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.call_key = Some(step_key.clone());
    }
    let fingerprint =
        scene_profile_stage_fingerprint(&step_key, prompt_name, root_field, &request)?;
    let fingerprint_sha = llm_call_checkpoint_fingerprint_sha256(&fingerprint)
        .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(fingerprint_sha);
    }
    let parse_output = |content: &str| {
        parse_scene_stage_root_array(content, prompt_name, root_field, context)
            .map_err(|error| AppError::LlmError(format!("{prompt_name} 输出不符合合同：{error}")))
    };
    if let Some(checkpoint) = load_completed_scene_profile_stage_checkpoint(
        state,
        workflow_run_id,
        &step_key,
        &fingerprint,
    )
    .await?
    {
        let output = parse_output(&checkpoint.response_content)?;
        let parsed_output = scene_profile_stage_parsed_output_json(root_field, &output);
        ensure_checkpoint_parsed_output_matches(
            &checkpoint.parsed_output,
            &parsed_output,
            &format!("场景档案同步 {step_key}"),
        )?;
        let item_count = output.as_array().map_or(0, Vec::len);
        response_chars.push(json!({
            "prompt_name": prompt_name,
            "response_chars": checkpoint.response_content.chars().count(),
            "source": "llm_call_checkpoint"}));
        report_scene_profile_sync_progress(
            progress,
            stage_progress.span.end(),
            format!("{prompt_name} 已从断点恢复：解析到 {item_count} 条{root_field}"),
        )?;
        return Ok(output);
    }

    let response_content = chat_scene_profile_stage(
        state,
        request,
        prompt_name,
        stage_progress,
        progress,
        response_chars,
    )
    .await?;
    let output = parse_output(&response_content)?;
    save_completed_scene_profile_stage_checkpoint(
        state,
        workflow_run_id,
        step_key,
        step_order,
        fingerprint,
        &response_content,
        scene_profile_stage_parsed_output_json(root_field, &output),
    )
    .await?;
    let item_count = output.as_array().map_or(0, Vec::len);
    report_scene_profile_sync_progress(
        progress,
        stage_progress.span.end(),
        format!("{prompt_name} 已完成：校验通过 {item_count} 条{root_field}"),
    )?;
    Ok(output)
}

fn scene_profile_stage_parsed_output_json(root_field: &str, output: &Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(root_field.to_string(), output.clone());
    Value::Object(object)
}
