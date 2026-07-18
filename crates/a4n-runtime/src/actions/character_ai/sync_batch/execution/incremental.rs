use crate::state::AppState;
use a4n_core::models::character::protagonist_profile;
use serde_json::{Value, json};

use super::super::super::CharacterAiProgress;
use super::super::super::sync_profile::sync_character_profile_from_chapters_with_scope;
use super::super::super::sync_scope::{profile_sync_skip_reason, sync_skip_summary};
use super::super::super::sync_types::{CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope};
use super::{character_sync_progress_base, character_sync_progress_total};

pub(crate) async fn sync_character_profile_models_to_chapter(
    state: &AppState,
    project_id: i64,
    target_chapter: i32,
    profiles: &[protagonist_profile::Model],
    progress: Option<&CharacterAiProgress<'_>>,
    cancelled_message: &str,
) -> Result<(Vec<Value>, Vec<Value>, Vec<Value>), String> {
    let mut synced = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    let total = character_sync_progress_total(profiles.len())?;
    for (index, profile) in profiles.iter().enumerate() {
        let current = character_sync_progress_base(index)?;
        let profile_progress = CharacterProgressScope::child(
            progress,
            current,
            CHARACTER_PROFILE_SYNC_STEPS,
            total,
            cancelled_message,
        );
        let before_message = format!(
            "角色 {}/{} · {}：准备同步",
            index + 1,
            profiles.len(),
            profile.name
        );
        profile_progress.report(0, CHARACTER_PROFILE_SYNC_STEPS, &before_message)?;
        if profile_sync_skip_reason(profile, target_chapter).is_some() {
            skipped.push(sync_skip_summary(profile, target_chapter));
            let after_message = format!(
                "角色 {}/{} · {}：无需同步到第 {} 章，跳过",
                index + 1,
                profiles.len(),
                profile.name,
                target_chapter
            );
            profile_progress.report(
                CHARACTER_PROFILE_SYNC_STEPS,
                CHARACTER_PROFILE_SYNC_STEPS,
                &after_message,
            )?;
            continue;
        }
        match sync_character_profile_from_chapters_with_scope(
            state,
            project_id,
            &profile.name,
            target_chapter,
            profile_progress,
        )
        .await
        {
            Ok(result) => synced.push(json!({
                "name": profile.name,
                "result": result,
            })),
            Err(error) if profile_progress.is_cancelled() => return Err(error),
            Err(error) => failed.push(json!({
                "name": profile.name,
                "error": error,
            })),
        }
        let after_message = format!(
            "角色 {}/{} · {}：处理完成",
            index + 1,
            profiles.len(),
            profile.name
        );
        profile_progress.report(
            CHARACTER_PROFILE_SYNC_STEPS,
            CHARACTER_PROFILE_SYNC_STEPS,
            &after_message,
        )?;
    }

    Ok((synced, skipped, failed))
}
