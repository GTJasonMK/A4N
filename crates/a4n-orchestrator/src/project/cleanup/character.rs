use a4n_core::{error::AppError, models::character::protagonist_profile};
use sea_orm::{ConnectionTrait, TransactionTrait};

use super::super::{
    ChapterDerivedCleanupArtifacts, chapter_cleanup_profile_state_should_restore_base,
};

pub(crate) async fn cleanup_character_profile_timeline_for_chapter_in<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<ChapterDerivedCleanupArtifacts, AppError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let profiles = a4n_db::project::protagonist_profiles_for_project_in(db, project_id)
        .await
        .map_err(crate::project::project_chapter_context_error_to_app_error)?;
    if profiles.is_empty() {
        return Ok(ChapterDerivedCleanupArtifacts::default());
    }

    let mut artifacts = ChapterDerivedCleanupArtifacts::default();

    for profile in profiles {
        if profile
            .first_seen_chapter()
            .is_some_and(|first_seen| first_seen >= chapter_number)
        {
            let deleted = a4n_db::character::delete_profile_cascade_transaction(db, profile.id)
                .await
                .map_err(crate::character::character_profile_delete_error)?;
            artifacts.extend_deleted_profile(deleted);
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
        .map_err(crate::character::character_db_error)?;
        artifacts.extend_character_timeline(deleted);

        if should_rewind {
            let restored_chapter = chapter_number - 1;
            let restored_state =
                profile_state_at_or_before_chapter(db, &profile, restored_chapter).await?;
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
            .map_err(crate::project::project_chapter_context_error_to_app_error)?;
        }
    }

    Ok(artifacts)
}

async fn profile_state_at_or_before_chapter<C>(
    db: &C,
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> Result<serde_json::Value, AppError>
where
    C: ConnectionTrait,
{
    if chapter_cleanup_profile_state_should_restore_base(profile, chapter_number) {
        return Ok(profile.base_attributes.clone());
    }

    let snapshot =
        a4n_db::project::snapshot_reference_for_profile_chapter(db, profile.id, chapter_number)
            .await
            .map_err(crate::project::project_chapter_context_error_to_app_error)?
            .ok_or_else(|| AppError::InvalidState {
                current: format!("角色 {} 缺少第 {} 章快照", profile.name, chapter_number),
                operation: "回滚角色档案状态".to_string(),
            })?;

    a4n_db::character::snapshot_attributes_from_data(&snapshot.data, chapter_number)
        .map_err(crate::character::character_db_error)
}
