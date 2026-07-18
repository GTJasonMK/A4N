use a4n_core::models::project::project_source_snapshot;
use chrono::Utc;
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::{persistence::create_source_snapshot_in_record, rules::normalize_chapter_scope};
use crate::project::{
    PROJECT_KIND_TEXT, ProjectMutationError, project_by_id_record,
    source_snapshot::DerivedChapterScope,
};

pub async fn create_source_snapshot_record(
    db: &DatabaseConnection,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<project_source_snapshot::Model, ProjectMutationError> {
    let root = project_by_id_record(db, root_project_id)
        .await?
        .ok_or_else(|| ProjectMutationError::NotFound(format!("项目不存在: {root_project_id}")))?;
    if root.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectMutationError::InvalidArgument(
            "只能为文本根项目创建源文本快照".to_string(),
        ));
    }
    let scope = normalize_chapter_scope(scope)?;
    let tx = db.begin().await.map_err(ProjectMutationError::Database)?;
    let snapshot =
        create_source_snapshot_in_record(&tx, root_project_id, scope, Utc::now().naive_utc())
            .await?;
    tx.commit().await.map_err(ProjectMutationError::Database)?;
    Ok(snapshot)
}
