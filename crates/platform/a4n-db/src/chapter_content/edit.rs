use a4n_db_models::models::chapter::chapter::{self, STATUS_DRAFT};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QuerySelect, Set};

use super::{ChapterContentEditError, assign_chapter_content};
use a4n_domain::chapter::EditChapterContentInput;

pub async fn save_chapter_content_edit_record<C>(
    db: &C,
    input: EditChapterContentInput,
) -> Result<chapter::Model, ChapterContentEditError>
where
    C: ConnectionTrait,
{
    if input.content.trim().is_empty() {
        return Err(ChapterContentEditError::InvalidArgument(
            "章节正文不能为空".to_string(),
        ));
    }
    let word_count = i32::try_from(input.content.chars().count()).map_err(|_| {
        ChapterContentEditError::InvalidArgument("章节正文字符数超出 i32 范围".to_string())
    })?;

    let status = chapter::Entity::find_by_id(input.chapter_id)
        .select_only()
        .column(chapter::Column::Status)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(ChapterContentEditError::database)?
        .ok_or_else(|| {
            ChapterContentEditError::NotFound(format!("章节不存在: {}", input.chapter_id))
        })?;

    if status == chapter::STATUS_GENERATING {
        return Err(ChapterContentEditError::InvalidState {
            current: status,
            operation: "保存章节正文".to_string(),
        });
    }

    let now = Utc::now().naive_utc();
    let mut active = chapter::ActiveModel {
        id: Set(input.chapter_id),
        ..Default::default()
    };
    assign_chapter_content(
        &mut active,
        input.content,
        word_count,
        STATUS_DRAFT,
        None,
        now,
    );
    if let Some(title) = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        active.title = Set(Some(title.to_string()));
    }
    active
        .update(db)
        .await
        .map_err(ChapterContentEditError::database)
}
