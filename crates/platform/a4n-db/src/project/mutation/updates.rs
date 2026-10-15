use super::contracts::{
    ProjectMutationError, SOURCE_SYNC_CURRENT, SOURCE_SYNC_ROOT_UPDATED, UpdateProjectRecordInput,
};
use super::rules::required_project_title;
use crate::project::project_record_from_model;
use a4n_db_models::models::project::{novel_project, source_snapshot_chapter};
use a4n_domain::project::ProjectRecord;
use chrono::{NaiveDateTime, Utc};
use sea_orm::ConnectionTrait;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, QueryTrait, Set,
};

pub async fn update_project_record(
    db: &impl crate::DbExecutor,
    input: UpdateProjectRecordInput,
) -> Result<ProjectRecord, ProjectMutationError> {
    let tx = db.begin().await?;
    let project = novel_project::Entity::find_by_id(input.project_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            ProjectMutationError::NotFound(format!("项目不存在: {}", input.project_id))
        })?;
    if project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(ProjectMutationError::InvalidArgument(
            "项目当前不可修改".into(),
        ));
    }
    if let Some(expected_updated_at) = input.expected_updated_at
        && project.updated_at != expected_updated_at
    {
        return Err(ProjectMutationError::InvalidArgument(
            "项目已变化，请刷新后重试".into(),
        ));
    }
    if let Some(interval) = input.fact_feedback_interval_chapters {
        if project.project_kind != "text" {
            return Err(ProjectMutationError::InvalidArgument(
                "只有文本项目可以配置正文事实归纳周期".into(),
            ));
        }
        crate::fact_feedback::update_feedback_interval(&tx, input.project_id, interval).await?;
    }
    let mut active: novel_project::ActiveModel = project.into();
    if let Some(title) = input.title {
        active.title = Set(required_project_title(&title)?.to_string());
    }
    if let Some(description) = input.description {
        active.description = Set(description.trim().to_string());
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(project_record_from_model(updated))
}

pub(crate) async fn mark_derived_projects_root_updated_in_record<C>(
    db: &C,
    root_project_id: i64,
    source_chapter_id: Option<i64>,
    now: NaiveDateTime,
) -> Result<u64, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let mut update = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::SourceSyncStatus,
            Expr::value(SOURCE_SYNC_ROOT_UPDATED),
        )
        .col_expr(novel_project::Column::UpdatedAt, Expr::value(now))
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .filter(novel_project::Column::SourceSyncStatus.eq(SOURCE_SYNC_CURRENT));
    if let Some(source_chapter_id) = source_chapter_id {
        update = update.filter(
            novel_project::Column::SourceSnapshotId.in_subquery(
                source_snapshot_chapter::Entity::find()
                    .select_only()
                    .column(source_snapshot_chapter::Column::SnapshotId)
                    .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
                    .into_query(),
            ),
        );
    }
    Ok(update.exec(db).await?.rows_affected)
}
