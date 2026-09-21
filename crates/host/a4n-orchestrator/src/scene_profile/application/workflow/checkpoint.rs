use super::super::types::{
    SCENE_ENTITY_STATE_PROMPT, SCENE_OCCURRENCE_PROMPT, SCENE_PROFILE_CANDIDATES_PROMPT,
    SCENE_TRANSITION_PROMPT, SceneProfileSyncRequestContext,
};
use crate::{OrchestratorContext, workflow::CompletedWorkflowCheckpointInput};
use a4n_domain::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

fn scene_final_step_key(context: &SceneProfileSyncRequestContext) -> String {
    format!(
        "scene_profile_sync.chapter.{}.final",
        context.chapter.chapter_number
    )
}

fn scene_final_step_order(chapter_index: usize) -> Result<i32, String> {
    let order = chapter_index
        .checked_mul(10)
        .and_then(|value| value.checked_add(9))
        .ok_or_else(|| "场景档案同步章节完成序号超出 usize 范围".to_string())?;
    i32::try_from(order).map_err(|_| "场景档案同步章节完成序号超出 i32 范围".to_string())
}

fn scene_final_fingerprint(context: &SceneProfileSyncRequestContext) -> Value {
    json!({
        "workflow": "scene_profile_sync",
        "stage": "chapter_final",
        "project_id": context.chapter.project_id,
        "chapter": {
            "id": context.chapter.id,
            "chapter_number": context.chapter.chapter_number,
            "content_sha256": source_content_sha256(&context.chapter.content),
            "analysis_data": context.chapter.analysis_data,
        },
        "prompt_templates": [
            SCENE_PROFILE_CANDIDATES_PROMPT,
            SCENE_OCCURRENCE_PROMPT,
            SCENE_TRANSITION_PROMPT,
            SCENE_ENTITY_STATE_PROMPT,
        ],
    })
}

pub(in crate::scene_profile::application) async fn has_completed_scene_final_checkpoint(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
) -> Result<bool, String> {
    let step_key = scene_final_step_key(context);
    let fingerprint = scene_final_fingerprint(context);
    state
        .load_completed_workflow_checkpoint(Some(workflow_run_id), &step_key, &fingerprint)
        .await
        .map(|checkpoint| checkpoint.is_some())
        .map_err(|error| error.to_string())
}

pub(in crate::scene_profile::application) async fn save_scene_final_checkpoint(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    chapter_index: usize,
) -> Result<(), String> {
    state
        .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow_run_id),
            step_key: scene_final_step_key(context),
            step_order: scene_final_step_order(chapter_index)?,
            input_fingerprint_json: scene_final_fingerprint(context),
            checkpoint_json: None,
        })
        .await
        .map_err(|error| error.to_string())
}
