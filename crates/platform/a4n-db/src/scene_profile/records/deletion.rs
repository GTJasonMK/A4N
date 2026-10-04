use a4n_db_models::models::{
    asset::profile_primary_material,
    scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};

pub async fn delete_scene_profile_records(
    db: &impl crate::DbExecutor,
    scene_profile_id: i64,
) -> SceneProfileDbResult<()> {
    let txn = db.begin().await?;
    let profile = scene_profile::Entity::find_by_id(scene_profile_id)
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| SceneProfileDbError::NotFound("场景档案不存在".to_string()))?;
    crate::chapter_visual_plan::ensure_visual_plan_scenes_unreferenced(&txn, &[profile.id]).await?;
    scene_entity_state::Entity::delete_many()
        .filter(scene_entity_state::Column::SceneProfileId.eq(profile.id))
        .exec(&txn)
        .await?;
    scene_transition::Entity::delete_many()
        .filter(
            scene_transition::Column::FromSceneProfileId
                .eq(profile.id)
                .or(scene_transition::Column::ToSceneProfileId.eq(profile.id)),
        )
        .exec(&txn)
        .await?;
    scene_occurrence::Entity::delete_many()
        .filter(scene_occurrence::Column::SceneProfileId.eq(profile.id))
        .exec(&txn)
        .await?;
    profile_primary_material::Entity::delete_many()
        .filter(profile_primary_material::Column::ProfileKind.eq("scene"))
        .filter(profile_primary_material::Column::ProfileId.eq(profile.id))
        .exec(&txn)
        .await?;
    crate::generation::delete_generation_run_targets_for(&txn, "scene_profile", &[profile.id])
        .await?;
    scene_profile::Entity::delete_by_id(profile.id)
        .exec(&txn)
        .await?;
    txn.commit().await?;
    Ok(())
}
