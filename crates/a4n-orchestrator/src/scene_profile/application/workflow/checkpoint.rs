use super::super::types::{
    SCENE_ENTITY_STATE_PROMPT, SCENE_OCCURRENCE_PROMPT, SCENE_PROFILE_CANDIDATES_PROMPT,
    SCENE_TRANSITION_PROMPT, SceneProfileSyncRequestContext,
};
use crate::{
    OrchestratorContext,
    scene_profile::SceneSyncResult,
    workflow::{
        CompletedWorkflowCheckpointInput, required_checkpoint_field, required_checkpoint_json,
    },
};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

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

pub(in crate::scene_profile::application) async fn load_completed_scene_final_checkpoint(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    _chapter_index: usize,
) -> Result<Option<Value>, String> {
    let step_key = scene_final_step_key(context);
    let fingerprint = scene_final_fingerprint(context);
    let Some(checkpoint) = state
        .load_completed_workflow_checkpoint(Some(workflow_run_id), &step_key, &fingerprint)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    required_checkpoint_json(&checkpoint, &step_key)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub(in crate::scene_profile::application) async fn save_scene_final_checkpoint(
    state: &OrchestratorContext,
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
    state
        .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow_run_id),
            step_key: scene_final_step_key(context),
            step_order: scene_final_step_order(chapter_index)?,
            input_fingerprint_json: scene_final_fingerprint(context),
            checkpoint_json: json!({
                "chapter_id": context.chapter.id,
                "chapter_number": context.chapter.chapter_number,
                "profile_ids": result.profile_ids,
                "profiles": profiles,
                "occurrence_count": result.occurrence_count,
                "transition_count": result.transition_count,
                "entity_state_count": result.entity_state_count,
            }),
        })
        .await
        .map_err(|error| error.to_string())
}

pub(in crate::scene_profile::application) fn merge_scene_final_checkpoint(
    checkpoint: Value,
    profile_ids: &mut BTreeSet<i64>,
    occurrence_count: &mut usize,
    transition_count: &mut usize,
    entity_state_count: &mut usize,
    profile_summaries: &mut BTreeMap<i64, Value>,
) -> Result<(), String> {
    for id in required_checkpoint_field(&checkpoint, "scene_profile_sync.final", "profile_ids")
        .map_err(|error| error.to_string())?
        .as_array()
        .ok_or_else(|| "scene_profile_sync.final.profile_ids 必须是数组".to_string())?
    {
        let id = id
            .as_i64()
            .ok_or_else(|| "scene_profile_sync.final.profile_ids[] 必须是整数".to_string())?;
        profile_ids.insert(id);
    }
    let profiles = required_checkpoint_field(&checkpoint, "scene_profile_sync.final", "profiles")
        .map_err(|error| error.to_string())?;
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
    let value = required_checkpoint_field(checkpoint, "scene_profile_sync.final", field)
        .map_err(|error| error.to_string())?
        .as_u64()
        .ok_or_else(|| format!("scene_profile_sync.final.{field} 必须是非负整数"))?;
    usize::try_from(value).map_err(|_| format!("scene_profile_sync.final.{field} 超出 usize 范围"))
}
