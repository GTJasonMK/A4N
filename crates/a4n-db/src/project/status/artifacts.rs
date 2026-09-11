use super::ProjectStatusReadError;
use crate::project::validate_positive_i64;
use a4n_db_models::models::chapter::chapter;
use a4n_domain::project::ProjectStatusArtifacts;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter,
    QuerySelect, Statement,
};

const STATUS_ARTIFACT_FLAGS_SQL: &str = r#"
SELECT
    EXISTS (SELECT 1 FROM novel_blueprint WHERE project_id = $1) AS has_blueprint,
    EXISTS (SELECT 1 FROM part_outline WHERE project_id = $1) AS has_part_outline,
    EXISTS (SELECT 1 FROM chapter_outline WHERE project_id = $1) AS has_chapter_outline
"#;

pub async fn project_status_artifacts(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectStatusArtifacts, ProjectStatusReadError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectStatusReadError::InvalidArgument)?;
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            STATUS_ARTIFACT_FLAGS_SQL,
            [project_id.into()],
        ))
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目状态产物", error))?
    else {
        return Err(ProjectStatusReadError::database(
            "读取项目状态产物",
            DbErr::RecordNotFound("项目状态产物查询没有返回结果".to_string()),
        ));
    };
    let has_blueprint: bool = row
        .try_get("", "has_blueprint")
        .map_err(|error| ProjectStatusReadError::database("读取项目蓝图", error))?;
    let has_part_outline: bool = row
        .try_get("", "has_part_outline")
        .map_err(|error| ProjectStatusReadError::database("读取项目分部大纲", error))?;
    let has_chapter_outline: bool = row
        .try_get("", "has_chapter_outline")
        .map_err(|error| ProjectStatusReadError::database("读取项目章节大纲", error))?;

    let has_chapter_content = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Content)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .into_tuple::<String>()
        .all(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目章节正文", error))?
        .into_iter()
        .any(|content| !content.trim().is_empty());

    Ok(ProjectStatusArtifacts {
        has_blueprint,
        has_part_outline,
        has_chapter_outline,
        has_chapter_content,
    })
}
