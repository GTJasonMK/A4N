use super::validate_positive_i64;
use a4n_db_models::models::project::novel_project;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QuerySelect,
};

#[derive(Debug, thiserror::Error)]
pub enum ProjectBasicRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("项目不存在: {project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("{operation}失败: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectBasicRecordError {
    fn database(operation: &'static str, source: DbErr) -> Self {
        Self::Database { operation, source }
    }
}

pub async fn project_import_analysis_progress(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Option<String>, ProjectBasicRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectBasicRecordError::InvalidArgument)?;
    let progress = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::ImportAnalysisProgress)
        .into_tuple::<Option<String>>()
        .one(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("读取项目导入分析进度", error))?
        .ok_or(ProjectBasicRecordError::ProjectNotFound { project_id })?;
    Ok(progress)
}

pub async fn save_project_import_analysis_progress(
    db: &DatabaseConnection,
    project_id: i64,
    progress: String,
) -> Result<(), ProjectBasicRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectBasicRecordError::InvalidArgument)?;
    let updated = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::ImportAnalysisProgress,
            Expr::value(Some(progress)),
        )
        .col_expr(
            novel_project::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(novel_project::Column::Id.eq(project_id))
        .exec(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("更新项目导入分析进度", error))?;
    if updated.rows_affected != 1 {
        return Err(ProjectBasicRecordError::ProjectNotFound { project_id });
    }
    Ok(())
}

pub async fn project_kind(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<String, ProjectBasicRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectBasicRecordError::InvalidArgument)?;
    let project_kind = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::ProjectKind)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("读取项目类型", error))?
        .ok_or(ProjectBasicRecordError::ProjectNotFound { project_id })?;
    Ok(project_kind)
}

pub async fn project_title(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<String, ProjectBasicRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectBasicRecordError::InvalidArgument)?;
    let title = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Title)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("读取项目标题", error))?
        .ok_or(ProjectBasicRecordError::ProjectNotFound { project_id })?;
    Ok(title)
}

pub(crate) async fn derived_project_count<C>(db: &C, root_project_id: i64) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find()
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .count(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn error_classifies_and_preserves_database_source() {
        let missing = ProjectBasicRecordError::ProjectNotFound { project_id: 17 };
        assert_eq!(missing.to_string(), "项目不存在: 17");
        assert!(missing.source().is_none());

        let database = ProjectBasicRecordError::database(
            "读取项目标题",
            DbErr::Custom("query failed".to_string()),
        );
        assert_eq!(
            database.to_string(),
            "读取项目标题失败: Custom Error: query failed"
        );
        assert!(matches!(
            &database,
            ProjectBasicRecordError::Database {
                operation: "读取项目标题",
                source: DbErr::Custom(message),
            } if message == "query failed"
        ));
        assert!(database.source().is_some());
    }

    #[tokio::test]
    async fn operations_reject_invalid_id_before_database_access() {
        let db = DatabaseConnection::Disconnected;

        let progress_error = project_import_analysis_progress(&db, 0)
            .await
            .expect_err("invalid project ID must fail before progress lookup");
        let save_error = save_project_import_analysis_progress(&db, 0, "{}".to_string())
            .await
            .expect_err("invalid project ID must fail before progress update");
        let kind_error = project_kind(&db, 0)
            .await
            .expect_err("invalid project ID must fail before kind lookup");
        let title_error = project_title(&db, 0)
            .await
            .expect_err("invalid project ID must fail before title lookup");

        for error in [progress_error, save_error, kind_error, title_error] {
            assert!(matches!(
                error,
                ProjectBasicRecordError::InvalidArgument(message)
                    if message == "project_id 必须大于 0"
            ));
        }
    }

    #[tokio::test]
    async fn save_progress_reports_missing_project_without_reading_full_row() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let error = save_project_import_analysis_progress(&db, i64::MAX, "{}".to_string())
            .await
            .expect_err("missing project must be reported by the narrow update");
        assert!(matches!(
            error,
            ProjectBasicRecordError::ProjectNotFound { project_id } if project_id == i64::MAX
        ));
    }
}
