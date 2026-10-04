use super::super::{ProjectMutationError, required_text, validate_positive_i64};
use super::record::{blueprint_character_record, blueprint_record, blueprint_relationship_record};
use a4n_db_models::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use a4n_domain::project::{BlueprintCharacterRecord, BlueprintRecord, BlueprintRelationshipRecord};
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter,
    QueryOrder, Statement,
};

const BLUEPRINT_SOURCE_REVIEW_SQL: &str = r#"
SELECT (
    EXISTS (
        SELECT 1 FROM "novel_project"
        WHERE "id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "chapter"
        WHERE "project_id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "chapter_outline"
        WHERE "project_id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "part_outline"
        WHERE "project_id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "blueprint_character"
        WHERE "project_id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "blueprint_relationship"
        WHERE "project_id" = $1 AND "created_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "protagonist_profile"
        WHERE "project_id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "object_profile"
        WHERE "project_id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "scene_profile"
        WHERE "project_id" = $1 AND "updated_at" > $2
    ) OR EXISTS (
        SELECT 1 FROM "foreshadowing_index"
        WHERE "project_id" = $1 AND "updated_at" > $2
    )
) AS "needs_review"
"#;

pub async fn blueprint_characters_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<BlueprintCharacterRecord>, DbErr>
where
    C: ConnectionTrait,
{
    blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
        .map(|rows| rows.into_iter().map(blueprint_character_record).collect())
}

pub async fn blueprint_relationships_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<BlueprintRelationshipRecord>, DbErr>
where
    C: ConnectionTrait,
{
    blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(db)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(blueprint_relationship_record)
                .collect()
        })
}

pub async fn novel_blueprint_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<BlueprintRecord>, DbErr>
where
    C: ConnectionTrait,
{
    novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map(|blueprint| blueprint.map(blueprint_record))
}

pub async fn blueprint_sources_need_review<C>(
    db: &C,
    project_id: i64,
    reviewed_at: NaiveDateTime,
) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            BLUEPRINT_SOURCE_REVIEW_SQL,
            [project_id.into(), reviewed_at.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("查询蓝图来源复核状态未返回结果".to_string()))?;
    row.try_get("", "needs_review")
}

pub async fn blueprint_versions_by_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<blueprint_version::Model>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)
}

pub async fn blueprint_version_by_project_version(
    db: &DatabaseConnection,
    project_id: i64,
    version: &str,
) -> Result<Option<blueprint_version::Model>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let version = required_text(version.to_string(), "version")
        .map_err(ProjectMutationError::InvalidArgument)?;
    blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .filter(blueprint_version::Column::Version.eq(version))
        .one(db)
        .await
        .map_err(ProjectMutationError::Database)
}
