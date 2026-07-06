use super::{GeneratePortraitInput, PortraitSummary, character_portrait_by_id, to_summary};
use crate::AppState;
use crate::actions::workflow_checkpoint::{
    load_completed_workflow_checkpoint, required_checkpoint_field, required_checkpoint_json,
};
use crate::file_utils::canonicalize_data_file_path;
use a4n_core::models::character::protagonist_profile;
use a4n_engine::llm::provider::ChatRequest;
use a4n_services::generation::context_aware_chat_request_trace_json;
use a4n_services::source::source_content_sha256;
use a4n_services::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};
use serde_json::{Value, json};

pub(super) fn portrait_generate_workflow_fingerprint(
    input: &GeneratePortraitInput,
    profile: &protagonist_profile::Model,
    expression: &str,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_generation",
        "mode": "full_body",
        "project_id": input.project_id,
        "profile": {
            "id": profile.id,
            "name": profile.name,
            "appearance": profile.appearance,
            "gender": profile.gender,
            "age": profile.age,
            "personality": profile.personality,
            "background": profile.background,
            "base_attributes": profile.base_attributes,
            "current_state": profile.current_state,
            "character_arc": profile.character_arc,
            "voice_fingerprint": profile.voice_fingerprint,
        },
        "style": input.style,
        "expression": expression,
        "pose": input.pose,
        "reference_image_ids": input.reference_image_ids,
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("立绘提示词请求序列化失败: {err}"))?,
    }))
}

pub(super) fn portrait_generate_business_key(fingerprint: &Value) -> Result<String, String> {
    let serialized = serde_json::to_string(fingerprint)
        .map_err(|err| format!("立绘工作流指纹序列化失败: {err}"))?;
    Ok(format!(
        "portrait_generation:{}",
        source_content_sha256(&serialized)
    ))
}

pub(super) fn portrait_prompt_checkpoint_fingerprint(
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_generation",
        "stage": "prompt",
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("立绘提示词请求序列化失败: {err}"))?,
    }))
}

pub(super) async fn load_completed_portrait_final_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    step_key: &str,
    fingerprint: &Value,
    project_id: i64,
) -> Result<Option<PortraitSummary>, String> {
    let Some(checkpoint) =
        load_completed_workflow_checkpoint(state, Some(workflow_run_id), step_key, fingerprint)
            .await?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_checkpoint_json(&checkpoint, step_key)?;
    let portrait_id = required_checkpoint_field(&checkpoint_json, step_key, "portrait_id")?
        .as_i64()
        .ok_or_else(|| format!("{step_key}.portrait_id 必须是整数"))?;
    let image_path = portrait_checkpoint_string(&checkpoint_json, step_key, "image_path")?;
    let portrait = character_portrait_by_id(state, portrait_id).await?;
    if portrait.project_id != project_id {
        return Err(format!(
            "{step_key} checkpoint 指向其他项目立绘: portrait_project_id={}",
            portrait.project_id
        ));
    }
    if portrait.image_path != image_path {
        return Err(format!(
            "{step_key} checkpoint 立绘路径不一致: checkpoint={}, db={}",
            image_path, portrait.image_path
        ));
    }
    canonicalize_data_file_path(&state.config.data_dir, &portrait.image_path, "断点立绘图片")?;
    Ok(Some(to_summary(&portrait)))
}

pub(super) fn portrait_checkpoint_string(
    checkpoint_json: &Value,
    step_key: &str,
    field: &str,
) -> Result<String, String> {
    required_checkpoint_field(checkpoint_json, step_key, field)?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| format!("{step_key}.{field} 必须是字符串"))
}

pub(super) async fn finalize_portrait_workflow<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<T, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .workflow_run
        .mark_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) fn portrait_image_workflow_fingerprint(
    workflow: &str,
    project_id: i64,
    source_portrait_id: i64,
    prompt: &str,
    reference_image_ids: &[i64],
    extra: Value,
) -> Value {
    json!({
        "workflow": workflow,
        "project_id": project_id,
        "source_portrait_id": source_portrait_id,
        "prompt": prompt,
        "reference_image_ids": reference_image_ids,
        "extra": extra,
    })
}

pub(super) fn portrait_image_business_key(
    workflow: &str,
    fingerprint: &Value,
) -> Result<String, String> {
    let serialized = serde_json::to_string(fingerprint)
        .map_err(|err| format!("立绘图片工作流指纹序列化失败: {err}"))?;
    Ok(format!("{workflow}:{}", source_content_sha256(&serialized)))
}

pub(super) fn missing_portrait_item_fingerprint(
    project_id: i64,
    profile: &protagonist_profile::Model,
    prompt: &str,
) -> Value {
    json!({
        "workflow": "missing_portrait_generation",
        "project_id": project_id,
        "profile": {
            "id": profile.id,
            "name": profile.name,
            "appearance": profile.appearance,
            "gender": profile.gender,
            "age": profile.age,
            "personality": profile.personality,
            "background": profile.background,
            "base_attributes": profile.base_attributes,
            "current_state": profile.current_state,
            "character_arc": profile.character_arc,
            "voice_fingerprint": profile.voice_fingerprint,
        },
        "prompt": prompt,
        "portrait_type": "half_body",
    })
}
