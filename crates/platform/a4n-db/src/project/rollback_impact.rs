mod contracts;

pub use contracts::ProjectRollbackImpactReadError;

use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use a4n_domain::project::{CleanupAction, ProjectRollbackImpactSummary};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, EntityTrait, QueryFilter,
    QuerySelect, Statement,
};

const BLUEPRINT_ROLLBACK_COUNTS_SQL: &str = r#"
SELECT
    (SELECT COUNT(*)::BIGINT FROM novel_blueprint WHERE project_id = $1) AS current_blueprint_count,
    (SELECT COUNT(*)::BIGINT FROM blueprint_version WHERE project_id = $1) AS version_count,
    (SELECT COUNT(*)::BIGINT FROM blueprint_character WHERE project_id = $1) AS character_count,
    (SELECT COUNT(*)::BIGINT FROM blueprint_relationship WHERE project_id = $1) AS relationship_count
"#;

const CHAPTER_ROLLBACK_COUNTS_SQL: &str = r#"
SELECT
    (SELECT COUNT(*)::BIGINT FROM chapter WHERE project_id = $1 AND status <> 'deleted') AS chapter_count,
    (SELECT MIN(chapter_number) FROM chapter WHERE project_id = $1 AND status <> 'deleted') AS chapter_start,
    (SELECT MAX(chapter_number) FROM chapter WHERE project_id = $1 AND status <> 'deleted') AS chapter_end,
    (SELECT COUNT(*)::BIGINT
       FROM generated_image
      WHERE project_id = $1
        AND chapter_id IN (SELECT id FROM chapter WHERE project_id = $1)) AS media_image_count,
    (SELECT COUNT(*)::BIGINT FROM generated_video WHERE project_id = $1) AS media_video_count,
    (SELECT COUNT(*)::BIGINT FROM task_queue WHERE project_id = $1) AS task_count,
    (SELECT COUNT(*)::BIGINT FROM object_profile WHERE project_id = $1) AS object_profile_count,
    (SELECT COUNT(*)::BIGINT FROM scene_profile WHERE project_id = $1) AS scene_profile_count,
    (SELECT COUNT(*)::BIGINT FROM scene_occurrence WHERE project_id = $1) AS scene_occurrence_count,
    (SELECT COUNT(*)::BIGINT FROM scene_transition WHERE project_id = $1) AS scene_transition_count,
    (SELECT COUNT(*)::BIGINT FROM scene_entity_state WHERE project_id = $1) AS scene_entity_state_count
"#;

pub async fn project_rollback_impacts(
    db: &DatabaseConnection,
    project_id: i64,
    actions: &[CleanupAction],
) -> Result<Vec<ProjectRollbackImpactSummary>, ProjectRollbackImpactReadError> {
    crate::project::validate_positive_i64(project_id, "project_id")
        .map_err(ProjectRollbackImpactReadError::InvalidArgument)?;

    let chapter_result = async {
        if actions.contains(&CleanupAction::DeleteChapters) {
            Ok(Some(chapter_rollback_impact(db, project_id).await?))
        } else {
            Ok(None)
        }
    };
    let chapter_outline_result = async {
        if actions.contains(&CleanupAction::DeleteChapterOutlines) {
            Ok(Some(chapter_outline_rollback_impact(db, project_id).await?))
        } else {
            Ok(None)
        }
    };
    let part_outline_result = async {
        if actions.contains(&CleanupAction::DeletePartOutlines) {
            Ok(Some(part_outline_rollback_impact(db, project_id).await?))
        } else {
            Ok(None)
        }
    };
    let blueprint_result = async {
        if actions.contains(&CleanupAction::DeleteBlueprintVersions) {
            Ok(Some(blueprint_rollback_impact(db, project_id).await?))
        } else {
            Ok(None)
        }
    };
    let (chapter_result, chapter_outline_result, part_outline_result, blueprint_result) = futures::join!(
        chapter_result,
        chapter_outline_result,
        part_outline_result,
        blueprint_result,
    );
    let mut impacts = Vec::with_capacity(actions.len());
    if let Some(impact) = chapter_result? {
        impacts.push(impact);
    }
    if let Some(impact) = chapter_outline_result? {
        impacts.push(impact);
    }
    if let Some(impact) = part_outline_result? {
        impacts.push(impact);
    }
    if let Some(impact) = blueprint_result? {
        impacts.push(impact);
    }
    Ok(impacts)
}

async fn chapter_rollback_impact(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectRollbackImpactSummary, ProjectRollbackImpactReadError> {
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            CHAPTER_ROLLBACK_COUNTS_SQL,
            [project_id.into()],
        ))
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("统计章节回滚影响失败", error))?
    else {
        return Err(ProjectRollbackImpactReadError::database(
            "统计章节回滚影响失败",
            DbErr::RecordNotFound("章节回滚影响计数查询没有返回结果".to_string()),
        ));
    };
    let chapter_count = count_i32_from_row(&row, "chapter_count", "读取项目章节失败", "章节数量")?;
    let chapter_start: Option<i32> = row
        .try_get("", "chapter_start")
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目章节失败", error))?;
    let chapter_end: Option<i32> = row
        .try_get("", "chapter_end")
        .map_err(|error| ProjectRollbackImpactReadError::database("读取项目章节失败", error))?;
    let media_image_count = count_i32_from_row(
        &row,
        "media_image_count",
        "统计章节生成图片失败",
        "章节生成图片数量",
    )?;
    let media_video_count = count_i32_from_row(
        &row,
        "media_video_count",
        "统计项目生成视频失败",
        "项目生成视频数量",
    )?;
    let task_count = count_i32_from_row(&row, "task_count", "统计项目任务失败", "项目任务数量")?;
    let object_profile_count = count_i32_from_row(
        &row,
        "object_profile_count",
        "统计项目对象档案失败",
        "项目对象档案数量",
    )?;
    let scene_profile_count = count_i32_from_row(
        &row,
        "scene_profile_count",
        "统计项目场景档案失败",
        "项目场景档案数量",
    )?;
    let scene_occurrence_count = count_i32_from_row(
        &row,
        "scene_occurrence_count",
        "统计项目场景出现记录失败",
        "项目场景出现记录数量",
    )?;
    let scene_transition_count = count_i32_from_row(
        &row,
        "scene_transition_count",
        "统计项目场景转场失败",
        "项目场景转场数量",
    )?;
    let scene_entity_state_count = count_i32_from_row(
        &row,
        "scene_entity_state_count",
        "统计项目场景实体状态失败",
        "项目场景实体状态数量",
    )?;

    Ok(ProjectRollbackImpactSummary {
        title: "删除章节正文与派生数据".to_string(),
        count: chapter_count,
        chapter_start,
        chapter_end,
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
    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            BLUEPRINT_ROLLBACK_COUNTS_SQL,
            [project_id.into()],
        ))
        .await
        .map_err(|error| ProjectRollbackImpactReadError::database("统计蓝图回滚影响失败", error))?
    else {
        return Err(ProjectRollbackImpactReadError::database(
            "统计蓝图回滚影响失败",
            DbErr::RecordNotFound("蓝图回滚影响计数查询没有返回结果".to_string()),
        ));
    };
    let counts = [
        count_i32_from_row(
            &row,
            "current_blueprint_count",
            "统计当前蓝图失败",
            "当前蓝图数量",
        )?,
        count_i32_from_row(&row, "version_count", "统计蓝图版本失败", "蓝图版本数量")?,
        count_i32_from_row(&row, "character_count", "统计蓝图角色失败", "蓝图角色数量")?,
        count_i32_from_row(
            &row,
            "relationship_count",
            "统计蓝图角色关系失败",
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

fn count_i32_from_row(
    row: &sea_orm::QueryResult,
    column: &str,
    operation: &'static str,
    label: &str,
) -> Result<i32, ProjectRollbackImpactReadError> {
    let count: i64 = row
        .try_get("", column)
        .map_err(|error| ProjectRollbackImpactReadError::database(operation, error))?;
    let count = u64::try_from(count)
        .map_err(|_| ProjectRollbackImpactReadError::InvalidArgument(format!("{label}为负数")))?;
    count_i32(count, label)
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
