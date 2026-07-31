use a4n_core::models::project::{novel_project, project_derivation};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::project::{
    PROJECT_KIND_TEXT,
    duplicate::{
        ProjectDuplicateError,
        shared::{ProjectCopyInsert, ProjectMediaFileCopier, insert_project_copy_record},
    },
};

use super::{
    super::workflow::copy_derived_workflow_records, source::duplicate_derived_source_snapshot,
};

pub(in crate::project::duplicate::derived) async fn copy_derived_project_graph_record<C, Copier>(
    db: &C,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<novel_project::Model, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: ProjectMediaFileCopier,
{
    if original.project_kind == PROJECT_KIND_TEXT {
        return Err(ProjectDuplicateError::InvalidArgument(
            "复制派生项目不能接收文本项目".to_string(),
        ));
    }
    let root_project_id =
        original
            .root_project_id
            .ok_or_else(|| ProjectDuplicateError::InvalidState {
                current: format!("派生项目 {} 缺少 root_project_id", original.id),
                operation: "复制派生项目".to_string(),
            })?;
    let source_snapshot_id =
        original
            .source_snapshot_id
            .ok_or_else(|| ProjectDuplicateError::InvalidState {
                current: format!("派生项目 {} 缺少 source_snapshot_id", original.id),
                operation: "复制派生项目".to_string(),
            })?;
    let derivation = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.eq(original.id))
        .one(db)
        .await
        .map_err(ProjectDuplicateError::Database)?
        .ok_or_else(|| ProjectDuplicateError::InvalidState {
            current: format!("派生项目 {} 缺少派生关系", original.id),
            operation: "复制派生项目".to_string(),
        })?;
    if derivation.root_project_id != root_project_id
        || derivation.source_snapshot_id != source_snapshot_id
        || derivation.derived_kind != original.project_kind
    {
        return Err(ProjectDuplicateError::InvalidState {
            current: format!("派生项目 {} 的派生关系与项目主体不一致", original.id),
            operation: "复制派生项目".to_string(),
        });
    }

    let duplicated_snapshot =
        duplicate_derived_source_snapshot(db, root_project_id, source_snapshot_id, now).await?;
    let new_snapshot_id = duplicated_snapshot.snapshot.id;
    let new_project = insert_project_copy_record(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            root_project_id: Some(root_project_id),
            parent_project_id: original.parent_project_id,
            source_snapshot_id: Some(new_snapshot_id),
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint: None,
            import_analysis_progress: None,
        },
        now,
    )
    .await?;

    project_derivation::ActiveModel {
        root_project_id: Set(root_project_id),
        derived_project_id: Set(new_project.id),
        derived_kind: Set(original.project_kind.clone()),
        source_snapshot_id: Set(new_snapshot_id),
        chapter_scope_json: Set(derivation.chapter_scope_json),
        creation_options_json: Set(derivation.creation_options_json),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectDuplicateError::Database)?;

    copy_derived_workflow_records(
        db,
        original.id,
        &new_project,
        root_project_id,
        source_snapshot_id,
        &duplicated_snapshot,
        now,
        media_file_copier,
    )
    .await?;

    Ok(new_project)
}
