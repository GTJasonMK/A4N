use super::ChapterService;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter::{self, STATUS_DRAFT};
use chrono::Utc;
use sea_orm::*;

impl ChapterService {
    /// 保存人工编辑后的章节正文。
    pub async fn save_content_edit(
        &self,
        chapter_id: i64,
        content: &str,
        title: Option<&str>,
    ) -> Result<chapter::Model, AppError> {
        if content.trim().is_empty() {
            return Err(AppError::InvalidArgument("章节正文不能为空".to_string()));
        }

        let ch = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;

        if ch.is_generating() {
            return Err(AppError::InvalidState {
                current: ch.status,
                operation: "保存章节正文".to_string(),
            });
        }

        let now = Utc::now().naive_utc();
        let word_count = content.chars().count() as i32;
        let mut active: chapter::ActiveModel = ch.into();
        active.content = Set(content.to_string());
        active.word_count = Set(word_count);
        active.status = Set(STATUS_DRAFT.to_string());
        active.summary = Set(None);
        active.style_tags = Set(None);
        active.analysis_data = Set(None);
        if let Some(title) = title.map(str::trim).filter(|s| !s.is_empty()) {
            active.title = Set(Some(title.to_string()));
        }
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}
