use crate::state::AppState;
use a4n_core::models::character::protagonist_profile;
use serde_json::{Value, json};

use super::super::super::CharacterAiProgress;
use super::super::super::sync_context::{
    CharacterSyncAnchorMode, build_character_sync_context_for_profile,
};
use super::super::super::sync_profile::sync_character_profile_context_with_scope;
use super::super::super::sync_scope::{
    CharacterSyncRangeMode, analysis_profile_sync_skip_reason, sync_skip_summary_with_reason,
};
use super::super::super::sync_types::{CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope};
use super::super::report_character_workflow_progress;
use super::{character_sync_progress_base, character_sync_progress_total};

pub(crate) async fn sync_analysis_character_profile_models_to_chapter_batched(
    state: &AppState,
    project_id: i64,
    target_chapter: i32,
    profiles: &[protagonist_profile::Model],
    progress: Option<&CharacterAiProgress<'_>>,
    cancelled_message: &str,
) -> Result<(Vec<Value>, Vec<Value>, Vec<Value>), String> {
    let chapters = state
        .orchestrator
        .character_sync_chapters(project_id)
        .await
        .map_err(|err| err.to_string())?;
    if chapters.is_empty() {
        return Err(format!("项目还没有章节: project_id={project_id}"));
    }

    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    let mut contexts = Vec::new();
    for profile in profiles {
        if analysis_profile_sync_skip_reason(profile, target_chapter).is_some() {
            skipped.push(sync_skip_summary_with_reason(
                profile,
                target_chapter,
                "target_before_first_seen",
            ));
            continue;
        }
        match build_character_sync_context_for_profile(
            state,
            profile.clone(),
            &chapters,
            target_chapter,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
            CharacterSyncAnchorMode::EnsureCurrent,
        )
        .await
        {
            Ok(context) => contexts.push(context),
            Err(error) => failed.push(json!({
                "name": profile.name,
                "error": error,
            })),
        }
    }

    if contexts.is_empty() {
        return Ok((Vec::new(), skipped, failed));
    }

    let total = character_sync_progress_total(contexts.len())?;
    report_character_workflow_progress(
        progress,
        0,
        total,
        "正在逐角色执行分阶段同步",
        cancelled_message,
    )?;
    let mut synced = Vec::new();
    for (index, context) in contexts.into_iter().enumerate() {
        let name = context.profile.name.clone();
        let base = character_sync_progress_base(index)?;
        let profile_progress = CharacterProgressScope::child(
            progress,
            base,
            CHARACTER_PROFILE_SYNC_STEPS,
            total,
            cancelled_message,
        );
        match sync_character_profile_context_with_scope(
            state,
            project_id,
            context,
            profile_progress,
        )
        .await
        {
            Ok(result) => synced.push(json!({
                "name": name,
                "result": result,
            })),
            Err(error) if profile_progress.is_cancelled() => return Err(error),
            Err(error) => failed.push(json!({
                "name": name,
                "error": error,
            })),
        }
    }

    Ok((synced, skipped, failed))
}
