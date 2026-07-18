use crate::{
    actions::{access, app_error_message},
    state::AppState,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::super::CharacterAiProgress;
use super::super::character_discovery::{character_identity_key, profile_character_keys};
use super::super::sync_scope::{
    analysis_sync_character_names, character_keys_from_names,
    character_names_from_chapter_analysis, character_names_from_discovery_result,
    resolve_character_sync_target_chapter,
};
use super::execution::sync_analysis_character_profile_models_to_chapter_batched;
use super::report_character_workflow_progress;

pub(crate) async fn sync_analysis_character_profiles_to_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    analysis: Option<&Value>,
    discovery_result: Option<&Value>,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    report_character_workflow_progress(
        progress,
        0,
        1,
        "正在按章节分析结果选择待同步角色",
        "分析后角色同步已停止",
    )?;
    let target_chapter =
        resolve_character_sync_target_chapter(state, project_id, chapter_number).await?;
    let analysis_character_names = analysis
        .map(character_names_from_chapter_analysis)
        .unwrap_or_default();
    let analysis_characters = analysis_character_names.iter().cloned().collect::<Vec<_>>();
    let discovery_character_names = discovery_result
        .map(character_names_from_discovery_result)
        .unwrap_or_default();
    let discovery_characters = discovery_character_names
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let sync_character_names =
        analysis_sync_character_names(&analysis_character_names, &discovery_character_names);
    let sync_characters = sync_character_names.iter().cloned().collect::<Vec<_>>();
    let sync_character_keys = character_keys_from_names(&sync_character_names);

    if sync_character_keys.is_empty() {
        return Ok(json!({
            "status": "skipped",
            "scope": "analysis_characters",
            "target_chapter": target_chapter,
            "analysis_character_count": 0,
            "analysis_characters": analysis_characters,
            "discovery_character_count": 0,
            "discovery_characters": discovery_characters,
            "sync_character_count": 0,
            "sync_characters": sync_characters,
            "matched": 0,
            "unmatched_analysis_characters": [],
            "unmatched_sync_characters": [],
            "total": 0,
            "synced": [],
            "skipped": [],
            "failed": [],
        }));
    }

    let profiles = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(app_error_message)?;
    let all_profile_keys = profiles
        .iter()
        .flat_map(|profile| profile_character_keys(profile).into_iter())
        .collect::<BTreeSet<_>>();
    let matched_profiles = profiles
        .into_iter()
        .filter(|profile| {
            profile_character_keys(profile)
                .iter()
                .any(|key| sync_character_keys.contains(key))
        })
        .collect::<Vec<_>>();
    let unmatched_analysis_characters = analysis_character_names
        .iter()
        .filter(|name| !all_profile_keys.contains(&character_identity_key(name)))
        .cloned()
        .collect::<Vec<_>>();
    let unmatched_sync_characters = sync_character_names
        .iter()
        .filter(|name| !all_profile_keys.contains(&character_identity_key(name)))
        .cloned()
        .collect::<Vec<_>>();

    if matched_profiles.is_empty() {
        return Ok(json!({
            "status": "skipped",
            "scope": "analysis_characters",
            "target_chapter": target_chapter,
            "analysis_character_count": analysis_character_names.len(),
            "analysis_characters": analysis_characters,
            "discovery_character_count": discovery_character_names.len(),
            "discovery_characters": discovery_characters,
            "sync_character_count": sync_character_names.len(),
            "sync_characters": sync_characters,
            "matched": 0,
            "unmatched_analysis_characters": unmatched_analysis_characters,
            "unmatched_sync_characters": unmatched_sync_characters,
            "total": 0,
            "synced": [],
            "skipped": [],
            "failed": [],
        }));
    }

    let (synced, skipped, failed) = sync_analysis_character_profile_models_to_chapter_batched(
        state,
        project_id,
        target_chapter,
        &matched_profiles,
        progress,
        "分析后角色同步已停止",
    )
    .await?;
    let status = if failed.is_empty() {
        if synced.is_empty() {
            "skipped"
        } else {
            "synced"
        }
    } else if synced.is_empty() && skipped.is_empty() {
        "failed"
    } else {
        "completed_with_errors"
    };

    Ok(json!({
        "status": status,
        "scope": "analysis_characters",
        "target_chapter": target_chapter,
        "analysis_character_count": analysis_character_names.len(),
        "analysis_characters": analysis_characters,
        "discovery_character_count": discovery_character_names.len(),
        "discovery_characters": discovery_characters,
        "sync_character_count": sync_character_names.len(),
        "sync_characters": sync_characters,
        "matched": matched_profiles.len(),
        "unmatched_analysis_characters": unmatched_analysis_characters,
        "unmatched_sync_characters": unmatched_sync_characters,
        "total": matched_profiles.len(),
        "synced": synced,
        "skipped": skipped,
        "failed": failed,
    }))
}
