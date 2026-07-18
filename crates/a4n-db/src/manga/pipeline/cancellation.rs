use crate::manga::{MANGA_PAGE_PROMPT_TYPE, MangaPipelineDbError};
use a4n_core::models::chapter::chapter_manga_prompt;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

/// 将 pending/generating 的整页提示词标为 cancelled，返回影响行数。
pub async fn cancel_manga_page_prompts(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<i64, MangaPipelineDbError> {
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .filter(chapter_manga_prompt::Column::Status.is_in(["pending", "generating"]))
        .all(db)
        .await?;
    let count = prompts.len() as i64;
    let now = Utc::now().naive_utc();
    for prompt in prompts {
        let mut active: chapter_manga_prompt::ActiveModel = prompt.into();
        active.status = Set("cancelled".to_string());
        active.updated_at = Set(now);
        active.update(db).await?;
    }
    Ok(count)
}
