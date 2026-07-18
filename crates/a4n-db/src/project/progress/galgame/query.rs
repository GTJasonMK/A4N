use super::pipeline::latest_galgame_pipeline_states_by_project;
use super::status::galgame_project_progress_from_counts;
use crate::project::progress::{ProjectProgressReadError, ProjectProgressSummary};
use a4n_core::models::galgame::{galgame_line, galgame_scene, galgame_script};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
};
use std::collections::HashMap;

pub(in crate::project::progress) async fn galgame_project_progress_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let scripts = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .column(galgame_script::Column::ProjectId)
        .filter(galgame_script::Column::ProjectId.is_in(project_ids.iter().copied()))
        .into_model::<GalgameScriptProgressRow>()
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("查询 Galgame 剧本失败", error))?;

    let mut script_id_by_project_id = HashMap::with_capacity(scripts.len());
    let mut project_id_by_script_id = HashMap::with_capacity(scripts.len());
    let mut script_ids = Vec::with_capacity(scripts.len());
    for script in scripts {
        script_ids.push(script.id);
        script_id_by_project_id.insert(script.project_id, script.id);
        project_id_by_script_id.insert(script.id, script.project_id);
    }

    let scenes = if script_ids.is_empty() {
        Vec::new()
    } else {
        galgame_scene::Entity::find()
            .select_only()
            .column(galgame_scene::Column::Id)
            .column(galgame_scene::Column::ScriptId)
            .filter(galgame_scene::Column::ScriptId.is_in(script_ids.iter().copied()))
            .into_model::<GalgameSceneProgressRow>()
            .all(db)
            .await
            .map_err(|error| ProjectProgressReadError::database("查询 Galgame 场景失败", error))?
    };

    let mut scene_counts_by_project_id: HashMap<i64, i64> = HashMap::new();
    let mut project_id_by_scene_id = HashMap::with_capacity(scenes.len());
    let mut scene_ids = Vec::with_capacity(scenes.len());
    for scene in scenes {
        if let Some(project_id) = project_id_by_script_id.get(&scene.script_id).copied() {
            scene_ids.push(scene.id);
            project_id_by_scene_id.insert(scene.id, project_id);
            *scene_counts_by_project_id.entry(project_id).or_insert(0) += 1;
        }
    }

    let line_counts_by_scene_id = line_counts_by_scene(db, &scene_ids).await?;
    let mut line_counts_by_project_id: HashMap<i64, i64> = HashMap::new();
    for (scene_id, line_count) in line_counts_by_scene_id {
        if let Some(project_id) = project_id_by_scene_id.get(&scene_id).copied() {
            *line_counts_by_project_id.entry(project_id).or_insert(0) += line_count;
        }
    }

    let pipeline_by_project_id = latest_galgame_pipeline_states_by_project(db, project_ids).await?;
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            galgame_project_progress_from_counts(
                script_id_by_project_id.contains_key(project_id),
                scene_counts_by_project_id
                    .get(project_id)
                    .copied()
                    .unwrap_or(0),
                line_counts_by_project_id
                    .get(project_id)
                    .copied()
                    .unwrap_or(0),
                pipeline_by_project_id
                    .get(project_id)
                    .cloned()
                    .unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

#[derive(Debug, FromQueryResult)]
struct GalgameScriptProgressRow {
    id: i64,
    project_id: i64,
}

#[derive(Debug, FromQueryResult)]
struct GalgameSceneProgressRow {
    id: i64,
    script_id: i64,
}

#[derive(Debug, FromQueryResult)]
struct GalgameLineCountRow {
    scene_id: i64,
    count: i64,
}

async fn line_counts_by_scene(
    db: &DatabaseConnection,
    scene_ids: &[i64],
) -> Result<HashMap<i64, i64>, ProjectProgressReadError> {
    if scene_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::SceneId)
        .column_as(galgame_line::Column::Id.count(), "count")
        .filter(galgame_line::Column::SceneId.is_in(scene_ids.iter().copied()))
        .group_by(galgame_line::Column::SceneId)
        .into_model::<GalgameLineCountRow>()
        .all(db)
        .await
        .map_err(|error| ProjectProgressReadError::database("统计 Galgame 台词失败", error))?;
    Ok(rows
        .into_iter()
        .map(|row| (row.scene_id, row.count.max(0)))
        .collect())
}
