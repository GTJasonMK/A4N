use super::super::contracts::ProjectRollbackImpactReadError;
use super::rules::rollback_count_i32;
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

#[derive(Debug, Clone, Default)]
pub(super) struct ProjectRollbackBlueprintFacts {
    pub(super) character_count: i32,
    pub(super) relationship_count: i32,
    pub(super) version_count: i32,
    pub(super) current_count: i32,
}

pub(super) async fn load_project_rollback_blueprint_facts(
    db: &DatabaseConnection,
    project_id: i64,
    include_blueprint_versions: bool,
) -> Result<ProjectRollbackBlueprintFacts, ProjectRollbackImpactReadError> {
    if !include_blueprint_versions {
        return Ok(ProjectRollbackBlueprintFacts::default());
    }

    Ok(ProjectRollbackBlueprintFacts {
        character_count: rollback_count_i32(
            blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计蓝图角色失败", error)
                })?,
            "蓝图角色数量",
        )?,
        relationship_count: rollback_count_i32(
            blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计蓝图角色关系失败", error)
                })?,
            "蓝图角色关系数量",
        )?,
        version_count: rollback_count_i32(
            blueprint_version::Entity::find()
                .filter(blueprint_version::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计蓝图版本失败", error)
                })?,
            "蓝图版本数量",
        )?,
        current_count: rollback_count_i32(
            novel_blueprint::Entity::find()
                .filter(novel_blueprint::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计当前蓝图失败", error)
                })?,
            "当前蓝图数量",
        )?,
    })
}
