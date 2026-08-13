mod contracts;

pub use contracts::ProjectRollbackImpactReadError;

use a4n_core::{
    models::{
        ai::{generated_image, generated_video},
        asset::object_profile,
        chapter::{chapter, chapter_outline, part_outline},
        project::{
            blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        },
        scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
        task::task_queue,
    },
    state::rollback::CleanupAction,
};
use a4n_domain::project::ProjectRollbackImpactSummary;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
};

pub async fn project_rollback_impacts(
    db: &DatabaseConnection,
    project_id: i64,
    actions: &[CleanupAction],
) -> Result<Vec<ProjectRollbackImpactSummary>, ProjectRollbackImpactReadError> {
    crate::project::validate_positive_i64(project_id, "project_id")
        .map_err(ProjectRollbackImpactReadError::InvalidArgument)?;

    let mut impacts = Vec::with_capacity(actions.len());
    if actions.contains(&CleanupAction::DeleteChapters) {
        impacts.push(chapter_rollback_impact(db, project_id).await?);
    }
    if actions.contains(&CleanupAction::DeleteChapterOutlines) {
        impacts.push(chapter_outline_rollback_impact(db, project_id).await?);
    }
    if actions.contains(&CleanupAction::DeletePartOutlines) {
        impacts.push(part_outline_rollback_impact(db, project_id).await?);
    }
    if actions.contains(&CleanupAction::DeleteBlueprintVersions) {
        impacts.push(blueprint_rollback_impact(db, project_id).await?);
    }
    Ok(impacts)
}

async fn chapter_rollback_impact(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectRollbackImpactSummary, ProjectRollbackImpactReadError> {
    let chapters = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ChapterNumber)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .into_tuple::<(i64, i32)>()
        .all(db)
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目章节失败", error))?;
    let chapter_ids = chapters.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let chapter_numbers = chapters
        .iter()
        .map(|(_, chapter_number)| *chapter_number)
        .collect::<Vec<_>>();
    let media_image_count = if chapter_ids.is_empty() {
        0
    } else {
        count_i32(
            generated_image::Entity::find()
                .filter(generated_image::Column::ProjectId.eq(project_id))
                .filter(generated_image::Column::ChapterId.is_in(chapter_ids))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计章节生成图片失败", error)
                })?,
            "章节生成图片数量",
        )?
    };
    let media_video_count = count_i32(
        generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目生成视频失败", error)
            })?,
        "项目生成视频数量",
    )?;
    let task_count = count_i32(
        task_queue::Entity::find()
            .filter(task_queue::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| ProjectRollbackImpactReadError::database("统计项目任务失败", error))?,
        "项目任务数量",
    )?;
    let object_profile_count = count_i32(
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目对象档案失败", error)
            })?,
        "项目对象档案数量",
    )?;
    let scene_profile_count = count_i32(
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景档案失败", error)
            })?,
        "项目场景档案数量",
    )?;
    let scene_occurrence_count = count_i32(
        scene_occurrence::Entity::find()
            .filter(scene_occurrence::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景出现记录失败", error)
            })?,
        "项目场景出现记录数量",
    )?;
    let scene_transition_count = count_i32(
        scene_transition::Entity::find()
            .filter(scene_transition::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景转场失败", error)
            })?,
        "项目场景转场数量",
    )?;
    let scene_entity_state_count = count_i32(
        scene_entity_state::Entity::find()
            .filter(scene_entity_state::Column::ProjectId.eq(project_id))
            .count(db)
            .await
            .map_err(|error| {
                ProjectRollbackImpactReadError::database("统计项目场景实体状态失败", error)
            })?,
        "项目场景实体状态数量",
    )?;

    Ok(ProjectRollbackImpactSummary {
        title: "删除章节正文与派生数据".to_string(),
        count: len_i32(chapter_numbers.len(), "章节数量")?,
        chapter_start: chapter_numbers.iter().min().copied(),
        chapter_end: chapter_numbers.iter().max().copied(),
        part_start: None,
        part_end: None,
        note: Some(format!(
            "将同步清理 {media_image_count} 张章节图片、{media_video_count} 条视频记录对应的本地文件、{object_profile_count} 个物体档案、{scene_profile_count} 个场景档案、{scene_occurrence_count} 条场景出场、{scene_transition_count} 条场景位移、{scene_entity_state_count} 条场景实体状态、项目 RAG 索引，并删除 {task_count} 个项目任务。"
        )),
    })
}

async fn chapter_outline_rollback_impact(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectRollbackImpactSummary, ProjectRollbackImpactReadError> {
    let chapter_numbers = chapter_outline::Entity::find()
        .select_only()
        .column(chapter_outline::Column::ChapterNumber)
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .into_tuple::<i32>()
        .all(db)
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目章节大纲失败", error))?;
    Ok(ProjectRollbackImpactSummary {
        title: "删除章节大纲".to_string(),
        count: len_i32(chapter_numbers.len(), "章节大纲数量")?,
        chapter_start: chapter_numbers.iter().min().copied(),
        chapter_end: chapter_numbers.iter().max().copied(),
        part_start: None,
        part_end: None,
        note: None,
    })
}

async fn part_outline_rollback_impact(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectRollbackImpactSummary, ProjectRollbackImpactReadError> {
    let part_numbers = part_outline::Entity::find()
        .select_only()
        .column(part_outline::Column::PartNumber)
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .into_tuple::<i32>()
        .all(db)
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目分部大纲失败", error))?;
    Ok(ProjectRollbackImpactSummary {
        title: "删除分部大纲".to_string(),
        count: len_i32(part_numbers.len(), "分部大纲数量")?,
        chapter_start: None,
        chapter_end: None,
        part_start: part_numbers.iter().min().copied(),
        part_end: part_numbers.iter().max().copied(),
        note: None,
    })
}

async fn blueprint_rollback_impact(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectRollbackImpactSummary, ProjectRollbackImpactReadError> {
    let counts = [
        count_i32(
            novel_blueprint::Entity::find()
                .filter(novel_blueprint::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计当前蓝图失败", error)
                })?,
            "当前蓝图数量",
        )?,
        count_i32(
            blueprint_version::Entity::find()
                .filter(blueprint_version::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计蓝图版本失败", error)
                })?,
            "蓝图版本数量",
        )?,
        count_i32(
            blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计蓝图角色失败", error)
                })?,
            "蓝图角色数量",
        )?,
        count_i32(
            blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .count(db)
                .await
                .map_err(|error| {
                    ProjectRollbackImpactReadError::database("统计蓝图角色关系失败", error)
                })?,
            "蓝图角色关系数量",
        )?,
    ];
    let count = counts.into_iter().try_fold(0_i32, |total, count| {
        total.checked_add(count).ok_or_else(|| {
            ProjectRollbackImpactReadError::InvalidArgument(
                "蓝图回滚影响总数超出 i32 范围".to_string(),
            )
        })
    })?;
    Ok(ProjectRollbackImpactSummary {
        title: "删除蓝图与版本".to_string(),
        count,
        chapter_start: None,
        chapter_end: None,
        part_start: None,
        part_end: None,
        note: Some("会重置蓝图、角色和关系版本数据。".to_string()),
    })
}

fn count_i32(count: u64, label: &str) -> Result<i32, ProjectRollbackImpactReadError> {
    i32::try_from(count).map_err(|_| {
        ProjectRollbackImpactReadError::InvalidArgument(format!("{label}超出 i32 范围"))
    })
}

fn len_i32(len: usize, label: &str) -> Result<i32, ProjectRollbackImpactReadError> {
    i32::try_from(len).map_err(|_| {
        ProjectRollbackImpactReadError::InvalidArgument(format!("回滚影响{label}超出 i32 范围"))
    })
}

#[cfg(test)]
mod tests;
