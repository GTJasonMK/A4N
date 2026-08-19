mod call;
mod checkpoint;

use serde_json::{Value, json};

use a4n_llm::provider::ChatRequest;

use crate::{
    OrchestratorContext, structured_llm::ensure_checkpoint_parsed_output_matches,
    workflow::llm_call_checkpoint_fingerprint_sha256,
};

use self::call::chat_scene_profile_stage;
use self::checkpoint::{
    load_completed_scene_profile_stage_checkpoint, save_completed_scene_profile_stage_checkpoint,
};
use super::super::parsing::parse_scene_stage_root_array;
use super::super::progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress};
use super::super::types::SceneProfileStageProgressSpec;
use super::request::scene_profile_stage_fingerprint;

pub(super) async fn run_scene_profile_stage_with_checkpoint(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    mut request: ChatRequest,
    prompt_name: &str,
    root_field: &str,
    step_key: String,
    step_order: i32,
    stage_progress: SceneProfileStageProgressSpec,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    response_chars: &mut Vec<Value>,
) -> Result<Value, String> {
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.call_key = Some(step_key.clone());
    }
    let fingerprint =
        scene_profile_stage_fingerprint(&step_key, prompt_name, root_field, &request)?;
    let fingerprint_sha =
        llm_call_checkpoint_fingerprint_sha256(&fingerprint).map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(fingerprint_sha);
    }
    if let Some(checkpoint) = load_completed_scene_profile_stage_checkpoint(
        state,
        workflow_run_id,
        &step_key,
        &fingerprint,
    )
    .await?
    {
        let output =
            parse_scene_stage_root_array(&checkpoint.response_content, prompt_name, root_field)?;
        let parsed_output = scene_profile_stage_parsed_output_json(root_field, &output);
        ensure_checkpoint_parsed_output_matches(
            &checkpoint.parsed_output,
            &parsed_output,
            &format!("场景档案同步 {step_key}"),
        )
        .map_err(|error| error.to_string())?;
        let item_count = output.as_array().map_or(0, Vec::len);
        response_chars.push(json!({
            "prompt_name": prompt_name,
            "response_chars": checkpoint.response_content.chars().count(),
            "source": "llm_call_checkpoint"}));
        report_scene_profile_sync_progress(
            progress,
            stage_progress.span.end(),
            format!("{prompt_name} 已从断点恢复：解析到 {item_count} 条{root_field}"),
            "场景档案同步已停止",
        )?;
        return Ok(output);
    }

    let call_result = chat_scene_profile_stage(
        state,
        request,
        prompt_name,
        root_field,
        stage_progress,
        progress,
        response_chars,
    )
    .await?;
    let output = call_result.output;
    save_completed_scene_profile_stage_checkpoint(
        state,
        workflow_run_id,
        step_key,
        step_order,
        fingerprint,
        &call_result.response_content,
        scene_profile_stage_parsed_output_json(root_field, &output),
    )
    .await?;
    Ok(output)
}

fn scene_profile_stage_parsed_output_json(root_field: &str, output: &Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(root_field.to_string(), output.clone());
    Value::Object(object)
}
