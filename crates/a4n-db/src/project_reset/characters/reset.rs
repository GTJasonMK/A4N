use super::{
    cleanup::{delete_retained_profile_related_rows, delete_text_discovered_profiles},
    contracts::{CharacterProfileResetScope, ResetCharacterProfilesResult},
    queries::load_character_profile_reset_scope,
};
use crate::{character, project_reset::ProjectResetDbError};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::project_reset) async fn reset_character_profiles_for_project<C>(
    tx: &C,
    project_id: i64,
    now: NaiveDateTime,
) -> Result<ResetCharacterProfilesResult, ProjectResetDbError>
where
    C: ConnectionTrait,
{
    let CharacterProfileResetScope {
        text_profile_ids,
        retained_profiles,
        retained_profile_ids,
        related_counts,
    } = load_character_profile_reset_scope(tx, project_id).await?;

    delete_retained_profile_related_rows(tx, &retained_profile_ids).await?;
    delete_text_discovered_profiles(tx, &text_profile_ids).await?;

    for profile in retained_profiles {
        let mut active =
            a4n_core::models::character::protagonist_profile::ActiveModel::from(profile.clone());
        character::apply_profile_current_state(&mut active, profile.base_attributes.clone());
        active.last_synced_chapter = Set(0);
        active.updated_at = Set(now);
        active.update(tx).await?;
    }

    Ok(ResetCharacterProfilesResult {
        reset_character_profiles: retained_profile_ids.len() as u64,
        deleted_character_profiles: text_profile_ids.len() as u64,
        deleted_character_snapshots: related_counts.character_snapshots,
        deleted_character_timeline_rows: related_counts.timeline_rows(),
        deleted_character_portraits: related_counts.character_portraits,
    })
}
