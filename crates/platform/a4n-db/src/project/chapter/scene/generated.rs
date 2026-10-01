use super::{ProjectChapterSceneWriteError, record::chapter_scene_record};
use a4n_db_models::models::chapter::{chapter, chapter_scene};
use a4n_domain::chapter::ProjectGeneratedChapterSceneSaveInput;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
};

pub async fn save_generated_chapter_scene_record(
    db: &DatabaseConnection,
    input: ProjectGeneratedChapterSceneSaveInput,
) -> Result<a4n_domain::chapter::ChapterSceneRecord, ProjectChapterSceneWriteError> {
    let word_count = i32::try_from(input.content.chars().count()).map_err(|_| {
        ProjectChapterSceneWriteError::InvalidArgument(
            "章节场景正文字符数超出 i32 范围".to_string(),
        )
    })?;
    let chapter_number = chapter::Entity::find_by_id(input.chapter_id)
        .select_only()
        .column(chapter::Column::ChapterNumber)
        .into_tuple::<i32>()
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectChapterSceneWriteError::NotFound(format!(
                "章节不存在: chapter_id={}",
                input.chapter_id
            ))
        })?;
    let now = Utc::now().naive_utc();

    let existing = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(input.chapter_id))
        .filter(chapter_scene::Column::SceneNumber.eq(input.scene_number))
        .one(db)
        .await?;

    if let Some(existing_scene) = existing {
        let mut active: chapter_scene::ActiveModel = existing_scene.into();
        active.content = Set(input.content);
        active.word_count = Set(word_count);
        active.updated_at = Set(now);
        return Ok(chapter_scene_record(active.update(db).await?));
    }

    let model = chapter_scene::ActiveModel {
        chapter_id: Set(input.chapter_id),
        scene_number: Set(input.scene_number),
        title: Set(Some(format!(
            "第{}章-场景{}",
            chapter_number, input.scene_number
        ))),
        content: Set(input.content),
        word_count: Set(word_count),
        summary: Set(None),
        scene_type: Set("development".to_string()),
        characters_present: Set(None),
        location: Set(None),
        timeline_position: Set(None),
        transition_from_previous: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(chapter_scene_record(model))
}
