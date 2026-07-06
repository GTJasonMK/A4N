use crate::{actions::access, state::AppState};
use a4n_core::models::character::protagonist_profile;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::character_discovery::{character_identity_key, profile_character_keys};
use super::sync_context::{CharacterSyncAnchorMode, build_character_sync_context_for_profile};
use super::sync_profile::{
    character_sync_preview_steps_for_context, sync_character_profile_context_with_scope,
    sync_character_profile_from_chapters_with_scope,
};
use super::sync_scope::{
    CharacterSyncRangeMode, analysis_profile_sync_skip_reason, analysis_sync_character_names,
    character_keys_from_names, character_names_from_chapter_analysis,
    character_names_from_discovery_result, is_cancelled_workflow_error, profile_sync_skip_reason,
    resolve_character_sync_target_chapter, sync_skip_summary, sync_skip_summary_with_reason,
};
use super::sync_types::{CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope};
use super::{CharacterAiPreviewStep, CharacterAiProgress};

fn report_character_workflow_progress(
    progress: Option<&CharacterAiProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.report(current, total, message, cancelled_message)?;
    }
    Ok(())
}

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
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

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
    let chapters = crate::workflow::character_sync_chapters(state, project_id).await?;
    if chapters.is_empty() {
        return Err(format!("项目还没有章节: project_id={project_id}"));
    }
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

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
        steps.extend(character_sync_preview_steps_for_context(
            state, project_id, &context, &name,
        )?);
    }

    Ok(steps)
}

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

    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

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

async fn sync_character_profile_models_to_chapter(
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
    let total = (profiles.len().max(1) as u32) * CHARACTER_PROFILE_SYNC_STEPS;
    for (index, profile) in profiles.iter().enumerate() {
        let current = (index as u32) * CHARACTER_PROFILE_SYNC_STEPS;
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
            Err(error) if is_cancelled_workflow_error(&error) => return Err(error),
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

async fn sync_analysis_character_profile_models_to_chapter_batched(
    state: &AppState,
    project_id: i64,
    target_chapter: i32,
    profiles: &[protagonist_profile::Model],
    progress: Option<&CharacterAiProgress<'_>>,
    cancelled_message: &str,
) -> Result<(Vec<Value>, Vec<Value>, Vec<Value>), String> {
    let chapters = crate::workflow::character_sync_chapters(state, project_id).await?;
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

    let total = (contexts.len().max(1) as u32) * CHARACTER_PROFILE_SYNC_STEPS;
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
        let base = index as u32 * CHARACTER_PROFILE_SYNC_STEPS;
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
            Err(error) if is_cancelled_workflow_error(&error) => return Err(error),
            Err(error) => failed.push(json!({
                "name": name,
                "error": error,
            })),
        }
    }

    Ok((synced, skipped, failed))
}
