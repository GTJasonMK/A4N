use super::contracts::{CharacterProfileRelatedCounts, CharacterProfileResetScope};
use crate::project_reset::ProjectResetDbError;
use a4n_core::models::character::{
    attribute_event, character_portrait, character_state_index, protagonist_behavior_record,
    protagonist_deletion_mark, protagonist_profile, snapshot_reference,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

pub(super) async fn load_character_profile_reset_scope<C>(
    tx: &C,
    project_id: i64,
) -> Result<CharacterProfileResetScope, ProjectResetDbError>
where
    C: ConnectionTrait,
{
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(tx)
        .await?;
    let (text_discovered_profiles, retained_profiles): (Vec<_>, Vec<_>) = profiles
        .into_iter()
        .partition(|profile| profile.is_text_discovered());
    let text_profile_ids = text_discovered_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let retained_profile_ids = retained_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let all_profile_ids = text_profile_ids
        .iter()
        .chain(retained_profile_ids.iter())
        .copied()
        .collect::<Vec<_>>();
    let related_counts = count_character_profile_related_rows(tx, &all_profile_ids).await?;

    Ok(CharacterProfileResetScope {
        text_profile_ids,
        retained_profiles,
        retained_profile_ids,
        related_counts,
    })
}

async fn count_character_profile_related_rows<C>(
    tx: &C,
    profile_ids: &[i64],
) -> Result<CharacterProfileRelatedCounts, ProjectResetDbError>
where
    C: ConnectionTrait,
{
    if profile_ids.is_empty() {
        return Ok(CharacterProfileRelatedCounts::default());
    }

    Ok(CharacterProfileRelatedCounts {
        attribute_events: attribute_event::Entity::find()
            .filter(attribute_event::Column::ProfileId.is_in(profile_ids.to_vec()))
            .count(tx)
            .await?,
        character_states: character_state_index::Entity::find()
            .filter(character_state_index::Column::ProfileId.is_in(profile_ids.to_vec()))
            .count(tx)
            .await?,
        behaviors: protagonist_behavior_record::Entity::find()
            .filter(protagonist_behavior_record::Column::ProfileId.is_in(profile_ids.to_vec()))
            .count(tx)
            .await?,
        deletion_marks: protagonist_deletion_mark::Entity::find()
            .filter(protagonist_deletion_mark::Column::ProfileId.is_in(profile_ids.to_vec()))
            .count(tx)
            .await?,
        character_snapshots: snapshot_reference::Entity::find()
            .filter(snapshot_reference::Column::ProfileId.is_in(profile_ids.to_vec()))
            .count(tx)
            .await?,
        character_portraits: character_portrait::Entity::find()
            .filter(character_portrait::Column::ProfileId.is_in(profile_ids.to_vec()))
            .count(tx)
            .await?,
    })
}
