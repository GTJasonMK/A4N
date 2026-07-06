use super::{
    SCENE_ENTITY_STATE_PROMPT, SCENE_OCCURRENCE_PROMPT, SCENE_PROFILE_CANDIDATES_PROMPT,
    SCENE_TRANSITION_PROMPT, SceneProfileSyncRequestContext,
};
use crate::actions::workflow_checkpoint::{
    load_completed_workflow_checkpoint, required_checkpoint_field, required_checkpoint_json,
    save_completed_workflow_checkpoint,
};
use crate::state::AppState;
use a4n_core::models::chapter::chapter;
use a4n_services::scene::SceneSyncResult;
use a4n_services::source::source_content_sha256;
use a4n_services::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn scene_profile_sync_business_key(
    project_id: i64,
    chapter_number: Option<i32>,
) -> String {
    match chapter_number {
        Some(chapter_number) => {
            format!("scene_profile_sync:{project_id}:chapter:{chapter_number}")
        }
        None => format!("scene_profile_sync:{project_id}:all"),
    }
}

pub(super) fn scene_profile_sync_workflow_fingerprint(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
) -> Value {
    json!({
        "workflow": "scene_profile_sync",
        "project_id": project_id,
        "chapter_number": chapter_number,
        "chapters": chapters.iter().map(|chapter| json!({
            "id": chapter.id,
            "chapter_number": chapter.chapter_number,
            "content_sha256": source_content_sha256(&chapter.content),
            "analysis_data": chapter.analysis_data,
        })).collect::<Vec<_>>(),
    })
}

pub(super) async fn finalize_scene_profile_sync_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Value, String>,
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

fn scene_final_step_key(context: &SceneProfileSyncRequestContext) -> String {
    format!(
        "scene_profile_sync.chapter.{}.final",
        context.chapter.chapter_number
    )
}

fn scene_final_step_order(chapter_index: usize) -> i32 {
    i32::try_from(chapter_index.saturating_mul(10).saturating_add(9)).unwrap_or(i32::MAX)
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

pub(super) async fn load_completed_scene_final_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    _chapter_index: usize,
) -> Result<Option<Value>, String> {
    let step_key = scene_final_step_key(context);
    let fingerprint = scene_final_fingerprint(context);
    let Some(checkpoint) =
        load_completed_workflow_checkpoint(state, Some(workflow_run_id), &step_key, &fingerprint)
            .await?
    else {
        return Ok(None);
    };
    required_checkpoint_json(&checkpoint, &step_key).map(Some)
}

pub(super) async fn save_scene_final_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    chapter_index: usize,
    result: &SceneSyncResult,
    profile_summaries: &BTreeMap<i64, Value>,
) -> Result<(), String> {
    let profiles = result
        .profile_ids
        .iter()
        .filter_map(|id| profile_summaries.get(id).cloned())
        .collect::<Vec<_>>();
    save_completed_workflow_checkpoint(
        state,
        Some(workflow_run_id),
        scene_final_step_key(context),
        scene_final_step_order(chapter_index),
        scene_final_fingerprint(context),
        json!({
            "chapter_id": context.chapter.id,
            "chapter_number": context.chapter.chapter_number,
            "profile_ids": result.profile_ids,
            "profiles": profiles,
            "occurrence_count": result.occurrence_count,
            "transition_count": result.transition_count,
            "entity_state_count": result.entity_state_count,
        }),
    )
    .await
}

pub(super) fn merge_scene_final_checkpoint(
    checkpoint: Value,
    profile_ids: &mut BTreeSet<i64>,
    occurrence_count: &mut usize,
    transition_count: &mut usize,
    entity_state_count: &mut usize,
    profile_summaries: &mut BTreeMap<i64, Value>,
) -> Result<(), String> {
    for id in required_checkpoint_field(&checkpoint, "scene_profile_sync.final", "profile_ids")?
        .as_array()
        .ok_or_else(|| "scene_profile_sync.final.profile_ids 必须是数组".to_string())?
    {
        let id = id
            .as_i64()
            .ok_or_else(|| "scene_profile_sync.final.profile_ids[] 必须是整数".to_string())?;
        profile_ids.insert(id);
    }
    let profiles = required_checkpoint_field(&checkpoint, "scene_profile_sync.final", "profiles")?;
    for profile in profiles
        .as_array()
        .ok_or_else(|| "scene_profile_sync.final.profiles 必须是数组".to_string())?
    {
        let id = profile
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| "scene_profile_sync.final.profiles[].id 必须是整数".to_string())?;
        profile_summaries.insert(id, profile.clone());
    }
    *occurrence_count += required_checkpoint_usize(&checkpoint, "occurrence_count")?;
    *transition_count += required_checkpoint_usize(&checkpoint, "transition_count")?;
    *entity_state_count += required_checkpoint_usize(&checkpoint, "entity_state_count")?;
    Ok(())
}

fn required_checkpoint_usize(checkpoint: &Value, field: &str) -> Result<usize, String> {
    let value = required_checkpoint_field(checkpoint, "scene_profile_sync.final", field)?
        .as_u64()
        .ok_or_else(|| format!("scene_profile_sync.final.{field} 必须是非负整数"))?;
    usize::try_from(value).map_err(|_| format!("scene_profile_sync.final.{field} 超出 usize 范围"))
}
