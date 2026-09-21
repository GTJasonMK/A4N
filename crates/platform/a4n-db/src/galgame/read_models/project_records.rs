use crate::galgame::GalgameProjectRecordError;
use a4n_db_models::models::galgame::{galgame_line, galgame_scene, galgame_script};
use sea_orm::{ConnectionTrait, EntityTrait};

pub async fn galgame_scene_in_project_record<C>(
    db: &C,
    project_id: i64,
    scene_id: i64,
) -> Result<galgame_scene::Model, GalgameProjectRecordError>
where
    C: ConnectionTrait,
{
    let (scene, script) = galgame_scene::Entity::find_by_id(scene_id)
        .find_also_related(galgame_script::Entity)
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameProjectRecordError::NotFound(format!("Galgame 场不存在: {scene_id}"))
        })?;
    let script = script.ok_or_else(|| {
        GalgameProjectRecordError::NotFound(format!("Galgame 剧本不存在: {}", scene.script_id))
    })?;
    if script.project_id != project_id {
        return Err(GalgameProjectRecordError::InvalidArgument(
            "Galgame 场不属于目标项目".to_string(),
        ));
    }
    Ok(scene)
}

pub async fn galgame_line_in_project_record<C>(
    db: &C,
    project_id: i64,
    line_id: i64,
) -> Result<galgame_line::Model, GalgameProjectRecordError>
where
    C: ConnectionTrait,
{
    let line = galgame_line::Entity::find_by_id(line_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            GalgameProjectRecordError::NotFound(format!("Galgame 行不存在: {line_id}"))
        })?;
    galgame_scene_in_project_record(db, project_id, line.scene_id).await?;
    Ok(line)
}
