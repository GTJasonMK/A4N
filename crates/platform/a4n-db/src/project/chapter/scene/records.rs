use super::{
    ProjectChapterSceneRecordError, ProjectChapterSceneUpdateInput,
    record::{chapter_scene_model_from_record, chapter_scene_record},
};
use crate::project::validate_positive_i64;
use a4n_db_models::models::chapter::{chapter, chapter_scene};
use a4n_domain::chapter::{ChapterRecord, ChapterSceneRecord};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub async fn chapter_by_project_number(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<ChapterRecord>, ProjectChapterSceneRecordError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectChapterSceneRecordError::InvalidArgument)?;
    if chapter_number <= 0 {
        return Err(ProjectChapterSceneRecordError::InvalidArgument(
            "chapter_number 必须大于 0".to_string(),
        ));
    }
    chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map(|chapter| chapter.map(crate::project::chapter_record))
        .map_err(|error| ProjectChapterSceneRecordError::database("按项目和章节号读取章节", error))
}

pub async fn chapter_scenes_by_chapter(
    db: &impl crate::DbExecutor,
    chapter_id: i64,
) -> Result<Vec<ChapterSceneRecord>, ProjectChapterSceneRecordError> {
    validate_positive_i64(chapter_id, "chapter_id")
        .map_err(ProjectChapterSceneRecordError::InvalidArgument)?;
    chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_scene::Column::SceneNumber)
        .all(db)
        .await
        .map(|scenes| scenes.into_iter().map(chapter_scene_record).collect())
        .map_err(|error| ProjectChapterSceneRecordError::database("读取章节场景列表", error))
}

pub async fn chapter_scene_by_id(
    db: &impl crate::DbExecutor,
    scene_id: i64,
) -> Result<Option<ChapterSceneRecord>, ProjectChapterSceneRecordError> {
    validate_positive_i64(scene_id, "scene_id")
        .map_err(ProjectChapterSceneRecordError::InvalidArgument)?;
    chapter_scene::Entity::find_by_id(scene_id)
        .one(db)
        .await
        .map(|scene| scene.map(chapter_scene_record))
        .map_err(|error| ProjectChapterSceneRecordError::database("读取章节场景", error))
}

pub async fn update_chapter_scene_record(
    db: &impl ConnectionTrait,
    input: ProjectChapterSceneUpdateInput,
) -> Result<ChapterSceneRecord, ProjectChapterSceneRecordError> {
    validate_positive_i64(input.scene.id, "scene_id")
        .map_err(ProjectChapterSceneRecordError::InvalidArgument)?;
    validate_positive_i64(input.scene.chapter_id, "chapter_id")
        .map_err(ProjectChapterSceneRecordError::InvalidArgument)?;
    if input.scene.scene_number <= 0 {
        return Err(ProjectChapterSceneRecordError::InvalidArgument(
            "scene_number 必须大于 0".to_string(),
        ));
    }
    let mut active: chapter_scene::ActiveModel =
        chapter_scene_model_from_record(&input.scene).into();

    if let Some(content) = input.content {
        let word_count = i32::try_from(content.chars().count()).map_err(|_| {
            ProjectChapterSceneRecordError::InvalidArgument(
                "章节场景正文字符数超出 i32 范围".to_string(),
            )
        })?;
        active.word_count = Set(word_count);
        active.content = Set(content);
    }
    if let Some(title) = input.title {
        active.title = Set(title);
    }
    if let Some(summary) = input.summary {
        active.summary = Set(summary);
    }
    if let Some(scene_type) = input.scene_type {
        active.scene_type = Set(scene_type);
    }
    if let Some(location) = input.location {
        active.location = Set(location);
    }
    if let Some(timeline_position) = input.timeline_position {
        active.timeline_position = Set(timeline_position);
    }
    active.updated_at = Set(input.updated_at);

    active
        .update(db)
        .await
        .map(chapter_scene_record)
        .map_err(|error| ProjectChapterSceneRecordError::database("更新章节场景", error))
}

pub async fn delete_chapter_scene_record(
    db: &impl ConnectionTrait,
    scene_id: i64,
) -> Result<u64, ProjectChapterSceneRecordError> {
    validate_positive_i64(scene_id, "scene_id")
        .map_err(ProjectChapterSceneRecordError::InvalidArgument)?;
    chapter_scene::Entity::delete_by_id(scene_id)
        .exec(db)
        .await
        .map(|result| result.rows_affected)
        .map_err(|error| ProjectChapterSceneRecordError::database("删除章节场景", error))
}
