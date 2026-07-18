use crate::scene_profile::SceneProfileDbResult;
use a4n_core::models::scene::{scene_entity_state, scene_occurrence, scene_transition};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub(in crate::scene_profile) async fn delete_chapter_scene_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> SceneProfileDbResult<()>
where
    C: ConnectionTrait,
{
    let occurrences = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::ChapterId.eq(chapter_id))
        .all(db)
        .await?;
    let occurrence_ids = occurrences.iter().map(|item| item.id).collect::<Vec<_>>();
    if !occurrence_ids.is_empty() {
        scene_entity_state::Entity::delete_many()
            .filter(scene_entity_state::Column::OccurrenceId.is_in(occurrence_ids.clone()))
            .exec(db)
            .await?;
        scene_transition::Entity::delete_many()
            .filter(
                scene_transition::Column::OccurrenceId
                    .is_in(occurrence_ids.clone())
                    .or(scene_transition::Column::ChapterId.eq(chapter_id)),
            )
            .exec(db)
            .await?;
        scene_occurrence::Entity::delete_many()
            .filter(scene_occurrence::Column::Id.is_in(occurrence_ids))
            .exec(db)
            .await?;
    } else {
        scene_transition::Entity::delete_many()
            .filter(scene_transition::Column::ProjectId.eq(project_id))
            .filter(scene_transition::Column::ChapterId.eq(chapter_id))
            .exec(db)
            .await?;
    }
    Ok(())
}
