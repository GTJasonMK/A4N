use super::super::contracts::{ProjectRollbackImpactQuery, ProjectRollbackImpactReadError};
use super::super::projection::rollback_impacts_from_facts;
use super::ProjectRollbackImpactFacts;
use super::blueprint::load_project_rollback_blueprint_facts;
use super::rules::rollback_count_i32;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    asset::object_profile,
    chapter::{chapter, chapter_outline, part_outline},
    scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
    task::task_queue,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

pub async fn project_rollback_impacts(
    db: &DatabaseConnection,
    query: ProjectRollbackImpactQuery,
) -> Result<
    Vec<super::super::contracts::ProjectRollbackImpactSummary>,
    ProjectRollbackImpactReadError,
> {
    crate::project::validate_positive_i64(query.project_id, "project_id")
        .map_err(ProjectRollbackImpactReadError::InvalidArgument)?;
    let project_id = query.project_id;

    let chapter_rows = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目章节失败", error))?;
    let chapter_ids: Vec<i64> = chapter_rows.iter().map(|row| row.id).collect();
    let chapter_numbers: Vec<i32> = chapter_rows.iter().map(|row| row.chapter_number).collect();

    let outline_rows = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目章节大纲失败", error))?;
    let outline_numbers: Vec<i32> = outline_rows.iter().map(|row| row.chapter_number).collect();

    let part_rows = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目分部大纲失败", error))?;
    let part_numbers: Vec<i32> = part_rows.iter().map(|row| row.part_number).collect();

    let media_image_count = if chapter_ids.is_empty() {
        0
    } else {
        let count = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .filter(generated_image::Column::ChapterId.is_in(chapter_ids))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计章节生成图片失败", error)
            })?;
        rollback_count_i32(count, "章节生成图片数量")?
    };
    let media_video_count = rollback_count_i32(
        generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目生成视频失败", error)
            })?,
        "项目生成视频数量",
    )?;
    let task_count = rollback_count_i32(
        task_queue::Entity::find()
            .filter(task_queue::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| ProjectRollbackImpactReadError::database("统计项目任务失败", error))?,
        "项目任务数量",
    )?;
    let object_profile_count = rollback_count_i32(
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目对象档案失败", error)
            })?,
        "项目对象档案数量",
    )?;
    let scene_profile_count = rollback_count_i32(
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景档案失败", error)
            })?,
        "项目场景档案数量",
    )?;
    let scene_occurrence_count = rollback_count_i32(
        scene_occurrence::Entity::find()
            .filter(scene_occurrence::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景出现记录失败", error)
            })?,
        "项目场景出现记录数量",
    )?;
    let scene_transition_count = rollback_count_i32(
        scene_transition::Entity::find()
            .filter(scene_transition::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景转场失败", error)
            })?,
        "项目场景转场数量",
    )?;
    let scene_entity_state_count = rollback_count_i32(
        scene_entity_state::Entity::find()
            .filter(scene_entity_state::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景实体状态失败", error)
            })?,
        "项目场景实体状态数量",
    )?;
    let blueprint_facts =
        load_project_rollback_blueprint_facts(db, project_id, query.delete_blueprint_versions)
            .await?;

    let facts = ProjectRollbackImpactFacts {
        chapter_numbers,
        outline_numbers,
        part_numbers,
        media_image_count,
        media_video_count,
        task_count,
        object_profile_count,
        scene_profile_count,
        scene_occurrence_count,
        scene_transition_count,
        scene_entity_state_count,
        blueprint_character_count: blueprint_facts.character_count,
        blueprint_relationship_count: blueprint_facts.relationship_count,
        blueprint_version_count: blueprint_facts.version_count,
        novel_blueprint_count: blueprint_facts.current_count,
    };

    rollback_impacts_from_facts(&query, &facts)
}
