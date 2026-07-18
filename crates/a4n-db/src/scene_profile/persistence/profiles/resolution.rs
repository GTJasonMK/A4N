use a4n_core::models::scene::scene_profile;
use sea_orm::{ConnectionTrait, EntityTrait};

use super::rules::merge_scene_profile_record_input;
use crate::scene_profile::records::{
    create_scene_profile_record, maybe_scene_profile_by_normalized_name,
    replace_scene_profile_record,
};
use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult, SceneProfileRecordDraft};

pub(super) async fn upsert_scene_profile_record<C>(
    db: &C,
    project_id: i64,
    draft: &SceneProfileRecordDraft,
) -> SceneProfileDbResult<scene_profile::Model>
where
    C: ConnectionTrait,
{
    match draft.scene_profile_id {
        Some(id) => {
            let existing = scene_profile::Entity::find_by_id(id)
                .one(db)
                .await?
                .ok_or_else(|| SceneProfileDbError::NotFound(format!("场景档案不存在: {id}")))?;
            if existing.project_id != project_id {
                return Err(SceneProfileDbError::InvalidArgument(format!(
                    "场景档案 #{id} 不属于项目 {project_id}"
                )));
            }
            replace_scene_profile_record(
                db,
                existing.clone(),
                merge_scene_profile_record_input(&existing, draft.input.clone()),
            )
            .await
        }
        None => {
            let existing = maybe_scene_profile_by_normalized_name(
                db,
                project_id,
                &draft.input.normalized_name,
            )
            .await?;
            match existing {
                Some(existing) => {
                    replace_scene_profile_record(
                        db,
                        existing.clone(),
                        merge_scene_profile_record_input(&existing, draft.input.clone()),
                    )
                    .await
                }
                None => create_scene_profile_record(db, project_id, draft.input.clone()).await,
            }
        }
    }
}
