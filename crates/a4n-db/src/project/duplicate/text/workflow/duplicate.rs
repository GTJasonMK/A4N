use a4n_core::models::project::novel_project;
use chrono::NaiveDateTime;
use sea_orm::ConnectionTrait;

use super::rules::parse_project_status;
use crate::project::duplicate::{
    ProjectDuplicateError,
    shared::{ProjectCopyInsert, insert_project_copy_record},
    text::{
        blueprint::{
            copy_project_blueprint_graph, load_project_blueprint_graph,
            project_blueprint_graph_raw_blueprint,
        },
        chapters::copy_project_chapters,
        outlines::{copy_project_chapter_outlines, copy_project_part_outlines},
        rules::project_copy_status_includes_blueprint,
    },
};

pub(super) async fn duplicate_text_project_data_record<C>(
    db: &C,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    now: NaiveDateTime,
) -> Result<novel_project::Model, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let original_status = parse_project_status(&original.status)?;
    let source_graph = load_project_blueprint_graph(db, original.id).await?;
    if project_copy_status_includes_blueprint(original_status) && source_graph.is_none() {
        return Err(ProjectDuplicateError::InvalidState {
            current: format!("源项目状态为 {} 且缺少蓝图数据", original.status),
            operation: "复制文本项目蓝图".to_string(),
        });
    }
    let raw_blueprint = match (&original.raw_blueprint, source_graph.as_ref()) {
        (Some(raw_blueprint), _) => Some(raw_blueprint.clone()),
        (None, Some(graph)) => Some(project_blueprint_graph_raw_blueprint(graph)?),
        (None, None) => None,
    };

    let new_project = insert_project_copy_record(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            root_project_id: original.root_project_id,
            parent_project_id: original.parent_project_id,
            source_snapshot_id: original.source_snapshot_id,
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint,
            import_analysis_progress: original.import_analysis_progress.clone(),
        },
        now,
    )
    .await?;

    if let Some(graph) = &source_graph {
        copy_project_blueprint_graph(db, new_project.id, graph, now).await?;
    }
    let part_id_map = copy_project_part_outlines(db, original.id, new_project.id, now).await?;
    let outline_id_map =
        copy_project_chapter_outlines(db, original.id, new_project.id, &part_id_map, now).await?;
    copy_project_chapters(db, original.id, new_project.id, &outline_id_map, now).await?;

    Ok(new_project)
}
