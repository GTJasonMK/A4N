use super::validate_positive_i64;
use a4n_core::models::project::novel_project;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set};

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
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("读取项目导入分析进度", error))?
        .ok_or(ProjectBasicRecordError::ProjectNotFound { project_id })?;
    Ok(project.import_analysis_progress)
}

pub async fn save_project_import_analysis_progress(
    db: &DatabaseConnection,
    project_id: i64,
    progress: String,
) -> Result<(), ProjectBasicRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectBasicRecordError::InvalidArgument)?;
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("保存导入分析进度前读取项目", error))?
        .ok_or(ProjectBasicRecordError::ProjectNotFound { project_id })?;

    let mut active: novel_project::ActiveModel = project.into();
    active.import_analysis_progress = Set(Some(progress));
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("更新项目导入分析进度", error))?;
    Ok(())
}

pub async fn project_kind(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<String, ProjectBasicRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectBasicRecordError::InvalidArgument)?;
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("读取项目类型", error))?
        .ok_or(ProjectBasicRecordError::ProjectNotFound { project_id })?;
    Ok(project.project_kind)
}

pub async fn project_title(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<String, ProjectBasicRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectBasicRecordError::InvalidArgument)?;
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|error| ProjectBasicRecordError::database("读取项目标题", error))?
        .ok_or(ProjectBasicRecordError::ProjectNotFound { project_id })?;
    Ok(project.title)
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
}
