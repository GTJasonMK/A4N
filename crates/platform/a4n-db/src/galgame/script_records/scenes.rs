use super::*;
use a4n_domain::galgame::GalgameSceneRecord;
use sea_orm::ConnectionTrait;

pub async fn update_galgame_scene_with_generation_cleanup_record(
    db: &impl crate::DbExecutor,
    project_id: i64,
    input: UpdateGalgameSceneRecordInput,
    delete_background: bool,
    delete_line_cgs: bool,
) -> Result<GalgameSceneRecord, GalgameSceneRecordError> {
    let scene_id = input.scene_id;
    let tx = db.begin().await?;
    if delete_background || delete_line_cgs {
        delete_galgame_scene_visual_generation_media_graph_record(
            &tx,
            project_id,
            scene_id,
            delete_background,
            delete_line_cgs,
        )
        .await?;
    }
    let updated = update_galgame_scene_record(&tx, input).await?;
    tx.commit().await?;
    Ok(super::mapping::scene_record(updated))
}

async fn update_galgame_scene_record<C>(
    db: &C,
    input: UpdateGalgameSceneRecordInput,
) -> Result<galgame_scene::Model, GalgameSceneRecordError>
where
    C: ConnectionTrait,
{
    let scene = galgame_scene::Entity::find_by_id(input.scene_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameSceneRecordError::NotFound(format!("Galgame 场不存在: {}", input.scene_id))
        })?;
    let mut active: galgame_scene::ActiveModel = scene.into();
    if let Some(title) = input.title {
        active.title = Set(title);
    }
    if let Some(location) = input.location {
        active.location = Set(location);
    }
    if let Some(time_of_day) = input.time_of_day {
        active.time_of_day = Set(time_of_day);
    }
    if let Some(background_note) = input.background_note {
        active.background_note = Set(background_note);
    }
    if let Some(next_scene_id) = input.next_scene_id {
        active.next_scene_id = Set(next_scene_id);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(GalgameSceneRecordError::Database)
}
