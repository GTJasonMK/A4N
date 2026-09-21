use a4n_db_models::models::galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GalgameScriptCleanupResult {
    pub(crate) deleted_scripts: u64,
    pub(crate) deleted_scenes: u64,
    pub(crate) deleted_lines: u64,
    pub(crate) deleted_choices: u64,
}

pub(crate) async fn delete_galgame_script_records_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<GalgameScriptCleanupResult, DbErr>
where
    C: ConnectionTrait,
{
    let script_ids = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if script_ids.is_empty() {
        return Ok(GalgameScriptCleanupResult::default());
    }

    let scene_ids = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .filter(galgame_scene::Column::ScriptId.is_in(script_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let line_ids = if scene_ids.is_empty() {
        Vec::new()
    } else {
        galgame_line::Entity::find()
            .select_only()
            .column(galgame_line::Column::Id)
            .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await?
    };

    let deleted_choices = if line_ids.is_empty() {
        0
    } else {
        galgame_choice::Entity::delete_many()
            .filter(galgame_choice::Column::LineId.is_in(line_ids.clone()))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_lines = if line_ids.is_empty() {
        0
    } else {
        galgame_line::Entity::delete_many()
            .filter(galgame_line::Column::Id.is_in(line_ids))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_scenes = if scene_ids.is_empty() {
        0
    } else {
        galgame_scene::Entity::delete_many()
            .filter(galgame_scene::Column::Id.is_in(scene_ids))
            .exec(db)
            .await?
            .rows_affected
    };
    let deleted_scripts = galgame_script::Entity::delete_many()
        .filter(galgame_script::Column::Id.is_in(script_ids))
        .exec(db)
        .await?
        .rows_affected;

    Ok(GalgameScriptCleanupResult {
        deleted_scripts,
        deleted_scenes,
        deleted_lines,
        deleted_choices,
    })
}
