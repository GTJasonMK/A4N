use super::*;

pub(crate) async fn delete_project_profile_scene_data<C>(
    db: &C,
    project_id: i64,
) -> Result<ProfileSceneCleanupResult, ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    use a4n_core::models::asset::{object_profile, profile_primary_material};
    use a4n_core::models::scene::{
        scene_entity_state, scene_occurrence, scene_profile, scene_transition,
    };

    let scene_profile_ids = scene_profile::Entity::find()
        .select_only()
        .column(scene_profile::Column::Id)
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let object_profile_ids = object_profile::Entity::find()
        .select_only()
        .column(object_profile::Column::Id)
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    generation::delete_generation_run_targets_for(db, "scene_profile", &scene_profile_ids).await?;
    generation::delete_generation_run_targets_for(db, "object_profile", &object_profile_ids)
        .await?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    let deleted_scene_entity_states = scene_entity_state::Entity::delete_many()
        .filter(scene_entity_state::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_scene_transitions = scene_transition::Entity::delete_many()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_scene_occurrences = scene_occurrence::Entity::delete_many()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_scene_profiles = scene_profile::Entity::delete_many()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_object_profiles = object_profile::Entity::delete_many()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;

    Ok(ProfileSceneCleanupResult {
        deleted_object_profiles,
        deleted_scene_profiles,
        deleted_scene_occurrences,
        deleted_scene_transitions,
        deleted_scene_entity_states,
    })
}
