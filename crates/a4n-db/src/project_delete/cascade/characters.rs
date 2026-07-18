use a4n_core::models::character::{
    attribute_event, character_portrait, character_state_index, foreshadowing_index,
    protagonist_behavior_record, protagonist_deletion_mark, protagonist_profile,
    snapshot_reference,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, QueryTrait};

use crate::generation;
use crate::project::{self as project_db, ProjectTextAnchorDeleteScope};
use crate::project_delete::{ProjectDeleteDbError, delete_project_profile_scene_data};

pub(super) async fn delete_project_character_data<C>(
    db: &C,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    let profile_ids = protagonist_profile::Entity::find()
        .select_only()
        .column(protagonist_profile::Column::Id)
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .into_query();
    let profile_id_values = protagonist_profile::Entity::find()
        .select_only()
        .column(protagonist_profile::Column::Id)
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;

    protagonist_behavior_record::Entity::delete_many()
        .filter(protagonist_behavior_record::Column::ProfileId.in_subquery(profile_ids.clone()))
        .exec(db)
        .await?;
    protagonist_deletion_mark::Entity::delete_many()
        .filter(protagonist_deletion_mark::Column::ProfileId.in_subquery(profile_ids.clone()))
        .exec(db)
        .await?;
    snapshot_reference::Entity::delete_many()
        .filter(snapshot_reference::Column::ProfileId.in_subquery(profile_ids.clone()))
        .exec(db)
        .await?;
    character_state_index::Entity::delete_many()
        .filter(character_state_index::Column::ProfileId.in_subquery(profile_ids.clone()))
        .exec(db)
        .await?;

    let portrait_ids = character_portrait::Entity::find()
        .select_only()
        .column(character_portrait::Column::Id)
        .filter(character_portrait::Column::ProfileId.in_subquery(profile_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    generation::delete_generation_run_targets_for(db, "character_portrait", &portrait_ids).await?;
    character_portrait::Entity::delete_many()
        .filter(character_portrait::Column::ProfileId.in_subquery(profile_ids.clone()))
        .exec(db)
        .await?;
    attribute_event::Entity::delete_many()
        .filter(attribute_event::Column::ProfileId.in_subquery(profile_ids.clone()))
        .exec(db)
        .await?;

    foreshadowing_index::Entity::delete_many()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    delete_project_profile_scene_data(db, project_id).await?;
    project_db::delete_project_text_anchors(db, project_id, ProjectTextAnchorDeleteScope::Project)
        .await?;
    generation::delete_generation_run_targets_for(db, "protagonist_profile", &profile_id_values)
        .await?;
    protagonist_profile::Entity::delete_many()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    Ok(())
}
