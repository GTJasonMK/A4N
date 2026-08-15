use a4n_core::models::project::novel_project;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

use crate::project::{
    PROJECT_KIND_TEXT,
    duplicate::{
        ProjectDuplicateError,
        shared::{ProjectCopyInsert, insert_project_copy_record},
        text::{
            blueprint::{copy_project_blueprint_graph, load_project_blueprint_graph},
            chapters::copy_project_chapters,
            outlines::{copy_project_chapter_outlines, copy_project_part_outlines},
        },
    },
};

pub async fn duplicate_text_project_record(
    db: &DatabaseConnection,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
) -> Result<novel_project::Model, ProjectDuplicateError> {
    if original.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectDuplicateError::InvalidArgument(
            "复制文本项目只能接收文本项目".to_string(),
        ));
    }

    let txn = db.begin().await?;
    let project = duplicate_text_project_data_record(
        &txn,
        original,
        user_id,
        new_title,
        Utc::now().naive_utc(),
    )
    .await?;
    txn.commit().await?;
    Ok(project)
}

async fn duplicate_text_project_data_record<C>(
    db: &C,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    now: NaiveDateTime,
) -> Result<novel_project::Model, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let source_graph = load_project_blueprint_graph(db, original.id).await?;
    let raw_blueprint = match (&original.raw_blueprint, &source_graph) {
        (Some(raw), Some(_)) => Some(raw.clone()),
        (None, None) => None,
        _ => {
            return Err(ProjectDuplicateError::InvalidState {
                current: format!(
                    "项目 {} 的 raw_blueprint 与正式蓝图图存在性不一致",
                    original.id
                ),
                operation: "复制文本项目蓝图".to_string(),
            });
        }
    };

    let new_project = insert_project_copy_record(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            root_project_id: original.root_project_id,
            source_snapshot_id: original.source_snapshot_id,
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint,
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
