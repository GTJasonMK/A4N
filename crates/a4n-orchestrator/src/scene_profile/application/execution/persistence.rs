use a4n_db::generation::RecordCompletedGenerationRunInput;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_progress::ProgressSpan;
use serde_json::{Value, json};

use crate::{OrchestratorContext, scene_profile::SceneProfileEvidenceRecordInput};

use super::super::parsing::parse_scene_profile_sync_content;
use super::super::progress::{
    SceneProfileSyncProgress, report_scene_profile_sync_progress, subspan,
};
use super::super::types::{
    SCENE_ENTITY_STATE_PROMPT, SCENE_OCCURRENCE_PROMPT, SCENE_PROFILE_CANDIDATES_PROMPT,
    SCENE_TRANSITION_PROMPT, SceneProfileSyncRequestContext, SceneProfileSyncStageResponses,
};
use super::super::workflow::save_scene_final_checkpoint;

pub(super) async fn persist_scene_profile_sync_chapter(
    state: &OrchestratorContext,
    project_id: i64,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    chapter_index: usize,
    chapter_span: ProgressSpan,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    stage_responses: SceneProfileSyncStageResponses,
) -> Result<(), String> {
    report_scene_profile_sync_progress(
        progress,
        subspan(chapter_span, 4, 5, 6).start(),
        format!(
            "第 {} 章模型已返回，正在校验场景档案结果",
            context.chapter.chapter_number
        ),
        "场景档案同步已停止",
    )?;
    let response_content = serde_json::to_string(&json!({
        "scenes": stage_responses.scenes,
        "occurrences": stage_responses.occurrences,
        "transitions": stage_responses.transitions,
        "entity_states": stage_responses.entity_states
    }))
    .map_err(|err| format!("场景档案同步阶段结果序列化失败: {err}"))?;
    let parsed = parse_scene_profile_sync_content(&response_content, context)?;
    report_scene_profile_sync_progress(
        progress,
        subspan(chapter_span, 5, 6, 6).start(),
        format!(
            "正在写入第 {} 章场景档案、出场和场景状态",
            context.chapter.chapter_number
        ),
        "场景档案同步已停止",
    )?;
    let request_json = stage_responses
        .requests
        .iter()
        .map(context_aware_chat_request_trace_json)
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(
        stage_responses
            .requests
            .iter()
            .map(|request| request.messages.clone())
            .collect::<Vec<_>>(),
    )
    .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let profile_evidence = parsed
        .profile_evidence
        .into_iter()
        .map(|item| SceneProfileEvidenceRecordInput {
            scene_ref: item.scene_ref,
            text_anchor_ids: item.evidence_anchor_ids,
        })
        .collect();
    state
        .replace_chapter_scene_sync_from_input_validated(
            project_id,
            context.chapter.id,
            context.chapter.chapter_number,
            a4n_db::project::ChapterSourceSnapshot::from(&context.chapter),
            parsed.input,
            profile_evidence,
            RecordCompletedGenerationRunInput {
                project_id,
                workflow: "scene_profile_sync".to_string(),
                step_key: "sync_scene_profiles_for_chapter".to_string(),
                title: "场景档案同步".to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: None,
                prompt_template_names_json: Some(json!([
                    SCENE_PROFILE_CANDIDATES_PROMPT,
                    SCENE_OCCURRENCE_PROMPT,
                    SCENE_TRANSITION_PROMPT,
                    SCENE_ENTITY_STATE_PROMPT
                ])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(messages_json),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "chapter_id": context.chapter.id,
                    "chapter_number": context.chapter.chapter_number,
                    "text_anchor_count": context.text_anchors.len(),
                    "existing_scene_ids": &context.existing_scene_ids,
                    "character_ids": &context.character_ids,
                    "object_ids": &context.object_ids
                })),
                source_refs_json: Some(json!({"chapter": {
                    "id": context.chapter.id,
                    "chapter_number": context.chapter.chapter_number,
                    "title": &context.chapter.title
                }})),
                media_refs_json: None,
                request_json: Some(request_json),
                response_summary_json: Some(json!({
                    "stages": stage_responses.response_chars,
                    "assembled_response_chars": response_content.chars().count()
                })),
                targets: Vec::new(),
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    save_scene_final_checkpoint(state, workflow_run_id, context, chapter_index).await?;
    Ok(())
}
