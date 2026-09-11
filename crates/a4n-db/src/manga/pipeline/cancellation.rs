use crate::manga::MangaPipelineDbError;
use a4n_db_models::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

/// 将 pending/generating 的整页提示词标为 cancelled，返回影响行数。
pub async fn cancel_manga_page_prompts(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<i64, MangaPipelineDbError> {
    let result = chapter_manga_prompt::Entity::update_many()
        .set(chapter_manga_prompt::ActiveModel {
            status: Set("cancelled".to_string()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        })
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .filter(chapter_manga_prompt::Column::Status.is_in(["pending", "generating"]))
        .exec(db)
        .await?;
    i64::try_from(result.rows_affected).map_err(|_| {
        MangaPipelineDbError::InvalidArgument("取消漫画页面数量超出 i64 范围".to_string())
    })
}
