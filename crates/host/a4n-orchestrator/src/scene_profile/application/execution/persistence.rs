use a4n_domain::{
    generation::RecordCompletedGenerationRunInput,
    scene_profile::{SceneProfileEvidenceRecordInput, SceneProfileSyncRecordsInput},
};
use a4n_error::AppError;
use a4n_llm_trace::context_aware_chat_request_trace_json;
use a4n_progress::ProgressSpan;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    scene_profile::{SceneProfileSyncHostPort, normalize_scene_sync_record_input},
};

use super::super::progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress};
use super::super::types::{
    SCENE_ENTITY_STATE_PROMPT, SCENE_OCCURRENCE_PROMPT, SCENE_PROFILE_CANDIDATES_PROMPT,
    SCENE_TRANSITION_PROMPT, SceneProfileSyncRequestContext, SceneProfileSyncStageResponses,
};
use super::super::workflow::save_scene_final_checkpoint;

pub(super) async fn persist_scene_profile_sync_chapter(
    state: &OrchestratorContext,
    host_port: &dyn SceneProfileSyncHostPort,
    user_id: i64,
    project_id: i64,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    chapter_index: usize,
    chapter_span: ProgressSpan,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    stage_responses: SceneProfileSyncStageResponses,
) -> Result<(), AppError> {
    report_scene_profile_sync_progress(
        progress,
        chapter_span.split(4, 6).start(),
        format!(
            "第 {} 章场景档案各阶段已校验通过，正在准备写入",
            context.chapter.chapter_number
        ),
    )?;
    report_scene_profile_sync_progress(
        progress,
        chapter_span.split(5, 6).start(),
        format!(
            "正在写入第 {} 章场景档案、出场和场景状态",
            context.chapter.chapter_number
        ),
    )?;
    let request_json = stage_responses
        .requests
        .iter()
        .map(context_aware_chat_request_trace_json)
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)?;
    let messages_json = serde_json::to_value(
        stage_responses
            .requests
            .iter()
            .map(|request| &request.messages)
            .collect::<Vec<_>>(),
    )?;
    let profile_evidence = stage_responses
        .profile_evidence
        .into_iter()
        .map(|item| SceneProfileEvidenceRecordInput {
            scene_ref: item.scene_ref,
            text_anchor_ids: item.evidence_anchor_ids,
        })
        .collect();
    let records = normalize_scene_sync_record_input(stage_responses.input)?;
    host_port
        .replace_chapter_scene_sync_records(
            user_id,
            SceneProfileSyncRecordsInput {
                project_id,
                chapter_id: context.chapter.id,
                chapter_number: context.chapter.chapter_number,
                source: a4n_db::project::chapter_source_snapshot_from_model(&context.chapter),
                records,
                profile_evidence,
                generation_run: RecordCompletedGenerationRunInput {
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
                        "stages": stage_responses.response_chars
                    })),
                    targets: Vec::new(),
                },
            },
        )
        .await?;
    save_scene_final_checkpoint(state, workflow_run_id, context, chapter_index).await?;
    Ok(())
}
