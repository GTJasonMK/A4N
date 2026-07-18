use crate::{
    actions::{access, app_error_message},
    state::AppState,
};
use serde_json::{Value, json};

use super::super::sync_context::{
    CharacterSyncAnchorMode, build_character_sync_context_for_profile,
};
use super::super::sync_profile::character_sync_preview_steps_for_context;
use super::super::sync_scope::{
    CharacterSyncRangeMode, profile_sync_skip_reason, resolve_character_sync_target_chapter,
};
use super::super::{CharacterAiPreviewStep, CharacterAiProgress};
use super::execution::sync_character_profile_models_to_chapter;
use super::report_character_workflow_progress;

pub async fn sync_all_character_profiles_to_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    report_character_workflow_progress(
        progress,
        0,
        1,
        "正在校验同步目标章节",
        "批量角色同步已停止",
    )?;
    let target_chapter =
        resolve_character_sync_target_chapter(state, project_id, chapter_number).await?;
    let profiles = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(app_error_message)?;

    if profiles.is_empty() {
        report_character_workflow_progress(
            progress,
            1,
            1,
            "当前没有可同步的角色档案",
            "批量角色同步已停止",
        )?;
        return Ok(json!({
            "status": "skipped",
            "target_chapter": target_chapter,
            "total": 0,
            "synced": [],
            "skipped": [],
            "failed": [],
        }));
    }

    let (synced, skipped, failed) = sync_character_profile_models_to_chapter(
        state,
        project_id,
        target_chapter,
        &profiles,
        progress,
        "批量角色同步已停止",
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
        "target_chapter": target_chapter,
        "total": profiles.len(),
        "synced": synced,
        "skipped": skipped,
        "failed": failed,
    }))
}

pub(crate) async fn sync_all_character_profiles_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let target_chapter =
        resolve_character_sync_target_chapter(state, project_id, chapter_number).await?;
    let chapters = state
        .orchestrator
        .character_sync_chapters(project_id)
        .await
        .map_err(|err| err.to_string())?;
    if chapters.is_empty() {
        return Err(format!("项目还没有章节: project_id={project_id}"));
    }
    let profiles = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(app_error_message)?;

    let mut steps = Vec::new();
    for profile in profiles {
        let name = profile.name.clone();
        if profile_sync_skip_reason(&profile, target_chapter).is_some() {
            steps.push(CharacterAiPreviewStep::status(
                format!("sync_{}", profile.id),
                format!("{name}：无需同步"),
                "character_sync_evidence_events",
                "reused",
                Some(format!("已同步到第 {target_chapter} 章或目标章节无需处理")),
            ));
            continue;
        }
        let context = build_character_sync_context_for_profile(
            state,
            profile,
            &chapters,
            target_chapter,
            CharacterSyncRangeMode::Incremental,
            CharacterSyncAnchorMode::RequireExistingCurrent,
        )
        .await?;
        steps.extend(
            character_sync_preview_steps_for_context(state, project_id, &context, &name).await?,
        );
    }

    Ok(steps)
}
