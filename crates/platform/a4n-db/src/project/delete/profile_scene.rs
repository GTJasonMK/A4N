use super::*;

pub(crate) async fn delete_project_profile_scene_data<C>(
    db: &C,
    project_id: i64,
    retained: &RetainedSourceResources,
) -> Result<ProfileSceneCleanupResult, ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    use a4n_db_models::models::asset::{object_profile, profile_primary_material};
    use a4n_db_models::models::scene::{
        scene_entity_state, scene_occurrence, scene_profile, scene_transition,
    };
    use sea_orm::sea_query::Expr;
    use serde_json::json;

    let scene_profile_ids = scene_profile::Entity::find()
        .select_only()
        .column(scene_profile::Column::Id)
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::Id.is_not_in(retained.scene_ids.iter().copied()))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let object_profile_ids = object_profile::Entity::find()
        .select_only()
        .column(object_profile::Column::Id)
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .filter(object_profile::Column::Id.is_not_in(retained.object_ids.iter().copied()))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    crate::chapter_visual_plan::ensure_visual_plan_scenes_unreferenced(db, &scene_profile_ids)
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
        .filter(scene_profile::Column::Id.is_in(scene_profile_ids))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_object_profiles = object_profile::Entity::delete_many()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .filter(object_profile::Column::Id.is_in(object_profile_ids))
        .exec(db)
        .await?
        .rows_affected;

    let now = chrono::Utc::now().naive_utc();
    scene_profile::Entity::update_many()
        .col_expr(
            scene_profile::Column::CurrentStateJson,
            Expr::value(json!({})),
        )
        .col_expr(scene_profile::Column::FirstSeenChapter, Expr::value(0))
        .col_expr(scene_profile::Column::LastSeenChapter, Expr::value(0))
        .col_expr(scene_profile::Column::EvidenceJson, Expr::value(json!([])))
        .col_expr(scene_profile::Column::UpdatedAt, Expr::value(now))
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::Id.is_in(retained.scene_ids.iter().copied()))
        .exec(db)
        .await?;
    object_profile::Entity::update_many()
        .col_expr(
            object_profile::Column::LocationContext,
            Expr::value(Option::<String>::None),
        )
        .col_expr(object_profile::Column::FirstSeenChapter, Expr::value(0))
        .col_expr(object_profile::Column::LastSeenChapter, Expr::value(0))
        .col_expr(object_profile::Column::EvidenceJson, Expr::value(json!([])))
        .col_expr(object_profile::Column::UpdatedAt, Expr::value(now))
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .filter(object_profile::Column::Id.is_in(retained.object_ids.iter().copied()))
        .exec(db)
        .await?;

    Ok(ProfileSceneCleanupResult {
        deleted_object_profiles,
        deleted_scene_profiles,
        deleted_scene_occurrences,
        deleted_scene_transitions,
        deleted_scene_entity_states,
    })
}
