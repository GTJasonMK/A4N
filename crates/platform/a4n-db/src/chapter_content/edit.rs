use a4n_db_models::models::chapter::chapter::{self, STATUS_DRAFT};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect, Set,
};

use super::{ChapterContentEditError, assign_chapter_content};
use a4n_domain::chapter::{ChapterRecord, EditChapterContentInput, chapter_edit_revision};

pub async fn save_chapter_content_edit_record(
    db: &DatabaseTransaction,
    input: EditChapterContentInput,
) -> Result<ChapterRecord, ChapterContentEditError> {
    if input.content.trim().is_empty() {
        return Err(ChapterContentEditError::InvalidArgument(
            "章节正文不能为空".to_string(),
        ));
    }
    let word_count = i32::try_from(input.content.chars().count()).map_err(|_| {
        ChapterContentEditError::InvalidArgument("章节正文字符数超出 i32 范围".to_string())
    })?;

    let current = chapter::Entity::find_by_id(input.chapter_id)
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .lock_exclusive()
        .one(db)
        .await
        .map_err(ChapterContentEditError::database)?
        .ok_or_else(|| {
            ChapterContentEditError::NotFound(format!("章节不存在: {}", input.chapter_id))
        })?;

    if current.status == chapter::STATUS_GENERATING {
        return Err(ChapterContentEditError::InvalidState {
            current: current.status,
            operation: "保存章节正文".to_string(),
        });
    }
    if input.expected_revision != chapter_edit_revision(&current.content, current.title.as_deref())
    {
        return Err(ChapterContentEditError::InvalidState {
            current: "章节已在其他窗口或任务中修改，请刷新并核对后再保存；当前草稿未被覆盖".into(),
            operation: "保存旧版本章节正文".into(),
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
    let updated = active
        .update(db)
        .await
        .map_err(ChapterContentEditError::database)?;
    Ok(crate::project::chapter_record(updated))
}
