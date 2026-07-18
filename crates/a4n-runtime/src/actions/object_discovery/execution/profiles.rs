use super::super::evidence::bind_object_profile_evidence;
use super::super::progress::{
    OBJECT_DISCOVERY_PROGRESS_TOTAL, ObjectDiscoveryProgress, report_object_discovery_progress,
};
use super::super::result::object_discovery_result;
use super::super::types::ObjectDiscoveryRequestContext;
use super::call::ObjectDiscoveryCallResult;
use crate::state::AppState;
use a4n_orchestrator::asset::normalize_object_profile_record_inputs;
use a4n_orchestrator::generation::RecordCompletedGenerationRunInput;
use serde_json::{Value, json};

pub(super) async fn persist_discovered_object_profiles(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    request_context: ObjectDiscoveryRequestContext,
    call_result: ObjectDiscoveryCallResult,
    progress: Option<&ObjectDiscoveryProgress<'_>>,
) -> Result<Value, String> {
    let ObjectDiscoveryRequestContext {
        chapters,
        text_anchors,
        request: _,
    } = request_context;
    let ObjectDiscoveryCallResult {
        parsed_drafts,
        response_content,
        llm_request,
        analyzed_chapters,
    } = call_result;
    let discovered_count = parsed_drafts.len();

    report_object_discovery_progress(progress, 90, "正在写入物体档案", "物体档案发现已停止")?;
    let request_json =
        serde_json::to_value(&llm_request).map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(&llm_request.messages)
        .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let record_inputs = normalize_object_profile_record_inputs(
        parsed_drafts
            .iter()
            .map(|item| item.draft.clone())
            .collect(),
    )
    .map_err(|err| err.to_string())?;
    let result = state
        .orchestrator
        .upsert_object_profiles_with_generation_run_record(
            project_id,
            record_inputs,
            RecordCompletedGenerationRunInput {
                project_id,
                workflow: "object_discovery".to_string(),
                step_key: "discover_object_profiles_for_chapters".to_string(),
                title: "物体档案发现".to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: None,
                prompt_template_names_json: Some(json!(["object_discovery"])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(messages_json),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "chapter_number": chapter_number,
                    "analyzed_chapters": analyzed_chapters,
                    "text_anchor_count": text_anchors.len(),
                })),
                source_refs_json: Some(json!({
                    "chapters": chapters.iter().map(|chapter| json!({
                        "id": chapter.id,
                        "chapter_number": chapter.chapter_number,
                        "title": chapter.title,
                    })).collect::<Vec<_>>(),
                })),
                media_refs_json: None,
                request_json: Some(request_json),
                response_summary_json: Some(json!({
                    "response_chars": response_content.chars().count(),
                })),
                targets: Vec::new(),
            },
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    bind_object_profile_evidence(state, project_id, &parsed_drafts, &result).await?;
    let value = object_discovery_result(state, chapters, discovered_count, result).await?;
    report_object_discovery_progress(
        progress,
        OBJECT_DISCOVERY_PROGRESS_TOTAL,
        "物体档案发现已完成",
        "物体档案发现已停止",
    )?;
    Ok(value)
}
