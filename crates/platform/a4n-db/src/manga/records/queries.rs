use super::mapping::manga_page_prompt_record_from_model;
use crate::manga::{
    MangaPromptRecordError, manga_record_database, require_positive_manga_id,
    validate_manga_page_number,
};
use a4n_db_models::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use a4n_domain::manga::MangaPagePromptRecord;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub async fn manga_page_prompts_by_chapter(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<MangaPagePromptRecord>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .all(db)
        .await
        .map_err(manga_record_database("读取章节漫画提示词失败"))?;
    Ok(prompts
        .into_iter()
        .map(manga_page_prompt_record_from_model)
        .collect())
}

pub async fn manga_page_prompt_statuses_by_chapter(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<(i64, String)>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .column(chapter_manga_prompt::Column::Status)
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .into_tuple::<(i64, String)>()
        .all(db)
        .await
        .map_err(manga_record_database("读取章节漫画提示词状态失败"))
}

pub async fn manga_page_prompt_statuses_by_chapters(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_ids: &[i64],
) -> Result<Vec<(i64, i64, String)>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    for chapter_id in chapter_ids {
        require_positive_manga_id(*chapter_id, "chapter_id")?;
    }
    if chapter_ids.is_empty() {
        return Ok(Vec::new());
    }
    chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::ChapterId)
        .column(chapter_manga_prompt::Column::Id)
        .column(chapter_manga_prompt::Column::Status)
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.is_in(chapter_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::ChapterId)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .into_tuple::<(i64, i64, String)>()
        .all(db)
        .await
        .map_err(manga_record_database("批量读取章节漫画提示词状态失败"))
}

pub async fn manga_page_prompt_for_image_generation(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
) -> Result<Option<MangaPagePromptRecord>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    validate_manga_page_number(page_number)?;
    chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .filter(chapter_manga_prompt::Column::PageNumber.eq(page_number))
        .one(db)
        .await
        .map_err(manga_record_database("读取待生成图片的漫画页面提示词失败"))
        .map(|prompt| prompt.map(manga_page_prompt_record_from_model))
}

pub async fn manga_page_prompt_by_id(
    db: &impl crate::DbExecutor,
    prompt_id: i64,
) -> Result<Option<MangaPagePromptRecord>, MangaPromptRecordError> {
    require_positive_manga_id(prompt_id, "prompt_id")?;
    chapter_manga_prompt::Entity::find_by_id(prompt_id)
        .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
        .one(db)
        .await
        .map_err(manga_record_database("按 ID 读取漫画提示词失败"))
        .map(|prompt| prompt.map(manga_page_prompt_record_from_model))
}
