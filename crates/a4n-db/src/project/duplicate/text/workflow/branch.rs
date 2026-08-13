use std::collections::HashMap;

use a4n_core::models::project::novel_project;
use chrono::NaiveDateTime;
use sea_orm::ConnectionTrait;

use a4n_core::state::validator::parse_project_status;

use super::rules::ensure_branch_status_allowed;
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
        rules::{
            project_copy_status_includes_blueprint, project_copy_status_includes_chapter_outlines,
            project_copy_status_includes_chapters, project_copy_status_includes_part_outlines,
        },
    },
};

pub(super) async fn branch_text_project_data_record<C>(
    db: &C,
    original: &novel_project::Model,
    branch_title: &str,
    branch_status: &str,
    now: NaiveDateTime,
) -> Result<novel_project::Model, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let original_status = parse_project_status(&original.status)
        .map_err(|error| ProjectDuplicateError::InvalidArgument(error.to_string()))?;
    let branch_status = parse_project_status(branch_status)
        .map_err(|error| ProjectDuplicateError::InvalidArgument(error.to_string()))?;
    ensure_branch_status_allowed(original_status, branch_status)?;

    let source_graph = load_project_blueprint_graph(db, original.id).await?;
    let should_copy_blueprint = project_copy_status_includes_blueprint(branch_status);
    if should_copy_blueprint && source_graph.is_none() {
        return Err(ProjectDuplicateError::InvalidState {
            current: format!("源项目状态为 {} 且缺少蓝图数据", branch_status.as_str()),
            operation: "创建包含蓝图的项目分支".to_string(),
        });
    }
    let raw_blueprint = if should_copy_blueprint {
        match (&original.raw_blueprint, source_graph.as_ref()) {
            (Some(raw_blueprint), _) => Some(raw_blueprint.clone()),
            (None, Some(graph)) => Some(project_blueprint_graph_raw_blueprint(graph)?),
            (None, None) => None,
        }
    } else {
        None
    };
    let current_chapter = if project_copy_status_includes_chapters(branch_status) {
        original.current_chapter
    } else {
        0
    };
    let total_chapters = if should_copy_blueprint {
        original.total_chapters
    } else {
        None
    };

    let new_project = insert_project_copy_record(
        db,
        original,
        ProjectCopyInsert {
            user_id: original.user_id,
            title: branch_title.to_string(),
            root_project_id: original.root_project_id,
            parent_project_id: original.parent_project_id,
            source_snapshot_id: original.source_snapshot_id,
            status: branch_status.as_str().to_string(),
            current_chapter,
            total_chapters,
            raw_blueprint,
            import_analysis_progress: original.import_analysis_progress.clone(),
        },
        now,
    )
    .await?;

    if should_copy_blueprint && let Some(graph) = &source_graph {
        copy_project_blueprint_graph(db, new_project.id, graph, now).await?;
    }
    let part_id_map = if project_copy_status_includes_part_outlines(branch_status) {
        copy_project_part_outlines(db, original.id, new_project.id, now).await?
    } else {
        HashMap::new()
    };
    let outline_id_map = if project_copy_status_includes_chapter_outlines(branch_status) {
        copy_project_chapter_outlines(db, original.id, new_project.id, &part_id_map, now).await?
    } else {
        HashMap::new()
    };
    if project_copy_status_includes_chapters(branch_status) {
        copy_project_chapters(db, original.id, new_project.id, &outline_id_map, now).await?;
    }

    Ok(new_project)
}
