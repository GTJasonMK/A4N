use crate::{character, generation, project_reset::ProjectResetDbError};
use a4n_db_models::models::character::{
    attribute_event, character_portrait, character_state_index, protagonist_behavior_record,
    protagonist_deletion_mark, snapshot_reference,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub(super) async fn delete_retained_profile_related_rows<C>(
    tx: &C,
    retained_profile_ids: &[i64],
) -> Result<(), ProjectResetDbError>
where
    C: ConnectionTrait,
{
    if retained_profile_ids.is_empty() {
        return Ok(());
    }

    attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await?;
    character_state_index::Entity::delete_many()
        .filter(character_state_index::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await?;
    protagonist_behavior_record::Entity::delete_many()
        .filter(protagonist_behavior_record::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await?;
    protagonist_deletion_mark::Entity::delete_many()
        .filter(protagonist_deletion_mark::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await?;
    snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await?;
    let portrait_ids = character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::Id)
        .filter(character_portrait::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .into_tuple::<i64>()
        .all(tx)
        .await?;
    generation::delete_generation_run_targets_for(tx, "character_portrait", &portrait_ids).await?;
    character_portrait::Entity::delete_many()
        .filter(character_portrait::Column::ProfileId.is_in(retained_profile_ids.to_vec()))
        .exec(tx)
        .await?;
    Ok(())
}

pub(super) async fn delete_text_discovered_profiles<C>(
    tx: &C,
    project_id: i64,
    text_profile_ids: &[i64],
) -> Result<(), ProjectResetDbError>
where
    C: ConnectionTrait,
{
    character::delete_profiles_cascade_in(tx, project_id, text_profile_ids).await?;
    Ok(())
}
