use crate::manga::{
    MangaPipelineStatusRows, MangaPromptRecordError, manga_record_database, require_manga_text,
    require_positive_manga_id, validate_manga_page_position,
};
use a4n_core::models::chapter::chapter_manga_prompt;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

pub async fn manga_page_prompts_by_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    page_prompt_type: &str,
) -> Result<Vec<chapter_manga_prompt::Model>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let page_prompt_type = require_manga_text(page_prompt_type, "page_prompt_type")?;
    chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(page_prompt_type))
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
        .all(db)
        .await
        .map_err(manga_record_database("读取章节漫画提示词失败"))
}

pub async fn manga_pipeline_status_rows(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    page_prompt_type: &str,
    pipeline_status_prompt_type: &str,
) -> Result<MangaPipelineStatusRows, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let page_prompt_type = require_manga_text(page_prompt_type, "page_prompt_type")?;
    let pipeline_status_prompt_type =
        require_manga_text(pipeline_status_prompt_type, "pipeline_status_prompt_type")?;
    let page_prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(page_prompt_type))
        .all(db)
        .await
        .map_err(manga_record_database("读取漫画页面提示词状态失败"))?;

    let status_row = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(pipeline_status_prompt_type))
        .order_by_desc(chapter_manga_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_manga_prompt::Column::Id)
        .one(db)
        .await
        .map_err(manga_record_database("读取漫画流水线状态失败"))?;

    Ok(MangaPipelineStatusRows {
        page_prompts,
        status_row,
    })
}

pub async fn manga_page_prompt_for_image_generation(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
    page_prompt_type: &str,
) -> Result<Option<chapter_manga_prompt::Model>, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    validate_manga_page_position(page_number, 0)?;
    let page_prompt_type = require_manga_text(page_prompt_type, "page_prompt_type")?;
    chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PromptType.eq(page_prompt_type))
        .filter(chapter_manga_prompt::Column::PageNumber.eq(page_number))
        .filter(chapter_manga_prompt::Column::PanelNumber.eq(0))
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
        .one(db)
        .await
        .map_err(manga_record_database("读取待生成图片的漫画页面提示词失败"))
}

pub async fn manga_page_prompt_by_id(
    db: &DatabaseConnection,
    prompt_id: i64,
) -> Result<Option<chapter_manga_prompt::Model>, MangaPromptRecordError> {
    require_positive_manga_id(prompt_id, "prompt_id")?;
    chapter_manga_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await
        .map_err(manga_record_database("按 ID 读取漫画提示词失败"))
}
