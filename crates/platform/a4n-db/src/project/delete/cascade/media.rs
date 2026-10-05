use a4n_db_models::models::{
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, QueryTrait};

use crate::project as project_db;
use crate::project::delete::ProjectDeleteDbError;

pub(super) async fn delete_project_media_and_source_data<C>(
    db: &C,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError>
where
    C: ConnectionTrait,
{
    delete_source_sync_records(db, project_id).await?;
    delete_video_project_records(db, project_id).await?;
    delete_galgame_records(db, project_id).await?;
    project_db::delete_project_generation_media_graph(db, project_id).await?;
    delete_canvas_project_records(db, project_id).await?;
    Ok(())
}

async fn delete_canvas_project_records(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError> {
    let graph_ids = canvas_graph::Entity::find()
        .select_only()
        .column(canvas_graph::Column::Id)
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .into_query();
    canvas_edge::Entity::delete_many()
        .filter(canvas_edge::Column::GraphId.in_subquery(graph_ids.clone()))
        .exec(db)
        .await?;
    canvas_node::Entity::delete_many()
        .filter(canvas_node::Column::GraphId.in_subquery(graph_ids))
        .exec(db)
        .await?;
    canvas_graph::Entity::delete_many()
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    canvas_text_asset::Entity::delete_many()
        .filter(canvas_text_asset::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    Ok(())
}

async fn delete_source_sync_records(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError> {
    let conflict_ids = source_sync_conflict::Entity::find()
        .select_only()
        .column(source_sync_conflict::Column::Id)
        .filter(source_sync_conflict::Column::DerivedProjectId.eq(project_id))
        .into_query();
    source_sync_decision::Entity::delete_many()
        .filter(source_sync_decision::Column::ConflictId.in_subquery(conflict_ids))
        .exec(db)
        .await?;
    source_sync_conflict::Entity::delete_many()
        .filter(source_sync_conflict::Column::DerivedProjectId.eq(project_id))
        .exec(db)
        .await?;
    source_sync_run::Entity::delete_many()
        .filter(source_sync_run::Column::DerivedProjectId.eq(project_id))
        .exec(db)
        .await?;
    Ok(())
}

async fn delete_video_project_records(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError> {
    let timeline_ids = assembly_timeline::Entity::find()
        .select_only()
        .column(assembly_timeline::Column::Id)
        .filter(assembly_timeline::Column::ProjectId.eq(project_id))
        .into_query();
    assembly_clip::Entity::delete_many()
        .filter(assembly_clip::Column::TimelineId.in_subquery(timeline_ids))
        .exec(db)
        .await?;
    assembly_timeline::Entity::delete_many()
        .filter(assembly_timeline::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    video_segment::Entity::delete_many()
        .filter(video_segment::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    Ok(())
}

async fn delete_galgame_records(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<(), ProjectDeleteDbError> {
    crate::galgame::delete_galgame_story_runtime_states_for_project(db, project_id).await?;
    crate::galgame::delete_galgame_story_graph_revisions(db, project_id).await?;
    let script_ids = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if script_ids.is_empty() {
        return Ok(());
    }

    let scene_ids = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .filter(galgame_scene::Column::ScriptId.is_in(script_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if !scene_ids.is_empty() {
        let line_ids = galgame_line::Entity::find()
            .select_only()
            .column(galgame_line::Column::Id)
            .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
            .into_tuple::<i64>()
            .all(db)
            .await?;
        if !line_ids.is_empty() {
            galgame_choice::Entity::delete_many()
                .filter(galgame_choice::Column::LineId.is_in(line_ids.clone()))
                .exec(db)
                .await?;
            galgame_line::Entity::delete_many()
                .filter(galgame_line::Column::Id.is_in(line_ids))
                .exec(db)
                .await?;
        }
        galgame_scene::Entity::delete_many()
            .filter(galgame_scene::Column::Id.is_in(scene_ids))
            .exec(db)
            .await?;
    }
    galgame_script::Entity::delete_many()
        .filter(galgame_script::Column::Id.is_in(script_ids))
        .exec(db)
        .await?;
    Ok(())
}
