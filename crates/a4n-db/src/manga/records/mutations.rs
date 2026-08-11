use crate::generation::delete_generation_run_targets_for;
use crate::manga::{
    MangaPipelineDbError, MangaPromptRecordError, MangaPromptUpdatedAt, manga_record_database,
    require_manga_text, require_positive_manga_id, validate_manga_page_prompt_layout,
};
use a4n_core::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use serde_json::Value;

pub async fn delete_manga_prompts_by_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<u64, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    let txn = db
        .begin()
        .await
        .map_err(manga_record_database("开启漫画提示词删除事务失败"))?;
    let prompt_ids = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .into_tuple::<i64>()
        .all(&txn)
        .await
        .map_err(manga_record_database("读取待删除漫画提示词 ID 失败"))?;
    delete_generation_run_targets_for(&txn, "chapter_manga_prompt", &prompt_ids).await?;
    let result = chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .exec(&txn)
        .await
        .map_err(manga_record_database("删除章节漫画提示词失败"))?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画提示词删除事务失败"))?;
    Ok(result.rows_affected)
}

pub async fn delete_manga_prompt_record(
    db: &DatabaseConnection,
    prompt_id: i64,
) -> Result<(), MangaPromptRecordError> {
    require_positive_manga_id(prompt_id, "prompt_id")?;
    let txn = db
        .begin()
        .await
        .map_err(manga_record_database("开启漫画提示词删除事务失败"))?;
    chapter_manga_prompt::Entity::find_by_id(prompt_id)
        .one(&txn)
        .await
        .map_err(manga_record_database("读取待删除漫画提示词失败"))?
        .ok_or_else(|| MangaPromptRecordError::NotFound("漫画提示词不存在".to_string()))?;
    delete_generation_run_targets_for(&txn, "chapter_manga_prompt", &[prompt_id]).await?;
    chapter_manga_prompt::Entity::delete_by_id(prompt_id)
        .exec(&txn)
        .await
        .map_err(manga_record_database("删除漫画提示词失败"))?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画提示词删除事务失败"))?;
    Ok(())
}

pub async fn update_manga_page_prompt_layout(
    db: &DatabaseConnection,
    page_prompt: chapter_manga_prompt::Model,
    layout: Value,
) -> Result<chapter_manga_prompt::Model, MangaPromptRecordError> {
    require_positive_manga_id(page_prompt.id, "prompt_id")?;
    if page_prompt.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err(MangaPromptRecordError::InvalidArgument(
            "只能更新 page_generation 的参考图绑定".to_string(),
        ));
    }
    validate_manga_page_prompt_layout(page_prompt.page_number, Some(&layout))
        .map_err(manga_pipeline_record_error)?;
    let mut active: chapter_manga_prompt::ActiveModel = page_prompt.into();
    active.layout = Set(Some(layout));
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(manga_record_database("更新漫画提示词布局失败"))
}

fn manga_pipeline_record_error(error: MangaPipelineDbError) -> MangaPromptRecordError {
    MangaPromptRecordError::InvalidArgument(error.to_string())
}

pub async fn update_manga_page_prompt_image_prompt(
    db: &DatabaseConnection,
    page_prompt: chapter_manga_prompt::Model,
    image_prompt: String,
) -> Result<chapter_manga_prompt::Model, MangaPromptRecordError> {
    require_positive_manga_id(page_prompt.id, "prompt_id")?;
    if page_prompt.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err(MangaPromptRecordError::InvalidArgument(
            "只能修改 page_generation 的图片提示词".to_string(),
        ));
    }
    require_manga_text(&image_prompt, "image_prompt")?;
    let mut active: chapter_manga_prompt::ActiveModel = page_prompt.into();
    active.image_prompt = Set(image_prompt);
    active.status = Set("pending".to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(manga_record_database("更新漫画图片提示词失败"))
}

pub async fn mark_manga_page_prompt_image_completed_with_image(
    db: &DatabaseConnection,
    page_prompt_id: i64,
    image_id: i64,
    updated_at: MangaPromptUpdatedAt,
) -> Result<(), MangaPromptRecordError> {
    require_positive_manga_id(page_prompt_id, "page_prompt_id")?;
    require_positive_manga_id(image_id, "image_id")?;
    let mut active_prompt = chapter_manga_prompt::ActiveModel {
        id: Set(page_prompt_id),
        ..Default::default()
    };
    active_prompt.generated_image_id = Set(Some(image_id));
    active_prompt.status = Set("completed".to_string());
    active_prompt.updated_at = Set(updated_at);
    active_prompt
        .update(db)
        .await
        .map_err(manga_record_database("回写漫画提示词生成图片失败"))?;
    Ok(())
}
