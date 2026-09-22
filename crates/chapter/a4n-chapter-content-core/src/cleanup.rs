use std::{collections::HashMap, fmt::Display};

use a4n_db_models::models::character::protagonist_profile;
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn cleanup_chapter_derived_data_in<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    cleanup_foreshadowing: bool,
    prune_ai_scene_profiles: bool,
    preserve_chapter_analysis_workflow_run_id: Option<i64>,
) -> Result<ChapterDerivedCleanupArtifacts, AppError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let mut artifacts = ChapterDerivedCleanupArtifacts::default();
    a4n_db::chapter_visual_plan::invalidate_live_chapter_visual_plans_for_source_change(
        db, project_id, chapter_id,
    )
    .await
    .map_err(|error| owner_error("失效章节视觉方案", error))?;
    a4n_db::text_anchor::delete_chapter_content_profile_evidence_records(
        db, project_id, chapter_id,
    )
    .await
    .map_err(|error| owner_error("清理章节正文证据", error))?;
    a4n_db::project::delete_automatic_chapter_outlines_for_source_change(
        db,
        project_id,
        chapter_number,
    )
    .await
    .map_err(AppError::Database)?;
    let character_artifacts =
        cleanup_character_profile_timeline_for_chapter_in(db, project_id, chapter_number).await?;
    artifacts
        .stale_profile_ids
        .extend(character_artifacts.stale_profile_ids);
    artifacts
        .stale_character_state_ids
        .extend(character_artifacts.stale_character_state_ids);
    artifacts
        .stale_attribute_event_ids
        .extend(character_artifacts.stale_attribute_event_ids);
    artifacts
        .media_file_paths
        .extend(character_artifacts.media_file_paths);
    artifacts.scene_records_changed = a4n_db::scene_profile::delete_chapter_scene_sync_records(
        db,
        project_id,
        chapter_id,
        prune_ai_scene_profiles,
    )
    .await
    .map_err(|error| owner_error("清理章节场景档案", error))?;

    if cleanup_foreshadowing {
        artifacts.changed_foreshadowing_ids =
            a4n_db::project::cleanup_foreshadowing_for_chapter(db, project_id, chapter_number)
                .await
                .map_err(|error| owner_error("清理章节伏笔", error))?;
    }
    a4n_db::workflow::invalidate_chapter_analysis_after_source_change(
        db,
        project_id,
        chapter_number,
        preserve_chapter_analysis_workflow_run_id,
    )
    .await
    .map_err(|error| owner_error("失效后续章节分析派生 checkpoint", error))?;
    Ok(artifacts)
}

async fn cleanup_character_profile_timeline_for_chapter_in<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<ChapterDerivedCleanupArtifacts, AppError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let profiles = a4n_db::project::protagonist_profiles_for_project_in(db, project_id)
        .await
        .map_err(|error| owner_error("读取角色档案", error))?;
    if profiles.is_empty() {
        return Ok(ChapterDerivedCleanupArtifacts::default());
    }

    let restored_chapter = chapter_number - 1;
    let snapshot_profile_ids = profiles
        .iter()
        .filter(|profile| {
            profile.last_synced_chapter >= chapter_number
                && profile
                    .first_seen_chapter()
                    .is_none_or(|first_seen| first_seen < chapter_number)
                && !profile_state_should_restore_base(profile, restored_chapter)
        })
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let snapshots = a4n_db::project::snapshot_references_for_profiles_chapter(
        db,
        &snapshot_profile_ids,
        restored_chapter,
    )
    .await
    .map_err(|error| owner_error("读取角色状态快照", error))?
    .into_iter()
    .map(|snapshot| (snapshot.profile_id, snapshot))
    .collect::<HashMap<_, _>>();

    let mut artifacts = ChapterDerivedCleanupArtifacts::default();
    for profile in profiles {
        if profile
            .first_seen_chapter()
            .is_some_and(|first_seen| first_seen >= chapter_number)
        {
            let deleted = a4n_db::character::delete_profile_cascade_transaction(db, profile.id)
                .await
                .map_err(|error| owner_error("删除章节新增角色档案", error))?;
            artifacts.stale_profile_ids.push(deleted.profile_id);
            artifacts
                .stale_character_state_ids
                .extend(deleted.state_index_ids);
            artifacts
                .stale_attribute_event_ids
                .extend(deleted.attribute_event_ids);
            artifacts.media_file_paths.extend(deleted.portrait_paths);
            continue;
        }

        let should_rewind = profile.last_synced_chapter >= chapter_number;
        let cleanup_to_chapter = if should_rewind {
            profile.last_synced_chapter
        } else {
            chapter_number
        };
        let deleted = a4n_db::character::delete_timeline_artifacts_from_chapter_checked(
            db,
            profile.id,
            chapter_number,
            Some(cleanup_to_chapter),
        )
        .await
        .map_err(|error| owner_error("清理角色时间线", error))?;
        artifacts
            .stale_character_state_ids
            .extend(deleted.state_index_ids);
        artifacts
            .stale_attribute_event_ids
            .extend(deleted.attribute_event_ids);

        if should_rewind {
            let restored_state = profile_state_at_or_before_chapter(
                &profile,
                restored_chapter,
                snapshots.get(&profile.id),
            )?;
            a4n_db::character::rewind_deletion_marks_after(db, profile.id, restored_chapter)
                .await
                .map_err(AppError::Database)?;
            let fields = a4n_db::character::profile_current_state_fields(restored_state);
            a4n_db::project::update_protagonist_profile_current_state(
                db,
                profile,
                a4n_db::project::ProtagonistProfileCurrentStateUpdate {
                    gender: fields.gender,
                    age: fields.age,
                    appearance: fields.appearance,
                    background: fields.background,
                    personality: fields.personality,
                    current_state: fields.current_state,
                },
                restored_chapter.max(0),
            )
            .await
            .map_err(|error| owner_error("恢复角色档案状态", error))?;
        }
    }
    Ok(artifacts)
}

fn profile_state_should_restore_base(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> bool {
    chapter_number <= 0
        || profile
            .first_seen_chapter()
            .is_some_and(|first_seen| chapter_number < first_seen)
}

fn profile_state_at_or_before_chapter(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
    snapshot: Option<&a4n_db_models::models::character::snapshot_reference::Model>,
) -> Result<serde_json::Value, AppError> {
    if profile_state_should_restore_base(profile, chapter_number) {
        return Ok(profile.base_attributes.clone());
    }
    let snapshot = snapshot.ok_or_else(|| AppError::InvalidState {
        current: format!("角色 {} 缺少第 {} 章快照", profile.name, chapter_number),
        operation: "回滚角色档案状态".to_string(),
    })?;
    a4n_db::character::snapshot_attributes_from_data(&snapshot.data, chapter_number)
        .map_err(|error| owner_error("读取角色状态快照", error))
}

fn owner_error(operation: &str, error: impl Display) -> AppError {
    AppError::InvalidState {
        current: error.to_string(),
        operation: operation.to_string(),
    }
}
