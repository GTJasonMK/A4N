use crate::generation::delete_generation_run_targets_for;
use crate::manga::{
    MangaPipelineDbError, MangaPromptRecordError, MangaPromptUpdatedAt, manga_record_database,
    require_manga_text, require_positive_manga_id, validate_manga_page_prompt_layout,
};
use a4n_db_models::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use a4n_db_models::models::chapter::manga_pipeline_artifact;
use a4n_domain::manga::MangaPagePromptRecord;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use serde_json::Value;

use super::mapping::manga_page_prompt_record_from_model;

pub async fn delete_manga_prompts_by_chapter(
    db: &impl crate::DbExecutor,
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
    manga_pipeline_artifact::Entity::delete_many()
        .filter(manga_pipeline_artifact::Column::ProjectId.eq(project_id))
        .filter(manga_pipeline_artifact::Column::ChapterId.eq(chapter_id))
        .exec(&txn)
        .await
        .map_err(manga_record_database("删除章节漫画阶段产物失败"))?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画提示词删除事务失败"))?;
    Ok(result.rows_affected)
}

pub async fn delete_manga_page_by_number(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
) -> Result<(), MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(chapter_id, "chapter_id")?;
    if page_number <= 0 {
        return Err(MangaPromptRecordError::InvalidArgument(
            "page_number 必须大于 0".into(),
        ));
    }
    let txn = db
        .begin()
        .await
        .map_err(manga_record_database("开启漫画页面删除事务失败"))?;
    let prompt_ids = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PageNumber.eq(page_number))
        .into_tuple::<i64>()
        .all(&txn)
        .await
        .map_err(manga_record_database("读取待删除漫画页面 ID 失败"))?;
    if prompt_ids.is_empty() {
        return Err(MangaPromptRecordError::NotFound(
            "漫画页面不存在或已删除".into(),
        ));
    }
    delete_generation_run_targets_for(&txn, "chapter_manga_prompt", &prompt_ids).await?;
    chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_manga_prompt::Column::PageNumber.eq(page_number))
        .exec(&txn)
        .await
        .map_err(manga_record_database("删除漫画页面失败"))?;
    manga_pipeline_artifact::Entity::delete_many()
        .filter(manga_pipeline_artifact::Column::ProjectId.eq(project_id))
        .filter(manga_pipeline_artifact::Column::ChapterId.eq(chapter_id))
        .filter(manga_pipeline_artifact::Column::PageNumber.eq(page_number))
        .exec(&txn)
        .await
        .map_err(manga_record_database("删除漫画页面阶段产物失败"))?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画页面删除事务失败"))
}

pub async fn update_manga_page_prompt_layout(
    db: &impl crate::DbExecutor,
    page_prompt: MangaPagePromptRecord,
    layout: Value,
) -> Result<MangaPagePromptRecord, MangaPromptRecordError> {
    require_positive_manga_id(page_prompt.id, "prompt_id")?;
    if page_prompt.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err(MangaPromptRecordError::InvalidArgument(
            "只能更新 page_generation 的参考图绑定".to_string(),
        ));
    }
    validate_manga_page_prompt_layout(page_prompt.page_number, Some(&layout))
        .map_err(manga_pipeline_record_error)?;
    let image_prompt = if layout.get("schema_version").and_then(Value::as_str)
        == Some("manga_page_generation_v4")
    {
        layout
            .get("pipeline")
            .and_then(|pipeline| pipeline.get("drawing_prompt"))
            .and_then(|prompt| prompt.get("image_prompt"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    } else {
        page_prompt.image_prompt.clone()
    };
    let updated = chapter_manga_prompt::ActiveModel {
        id: Set(page_prompt.id),
        image_prompt: Set(image_prompt),
        layout: Set(Some(layout)),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(db)
    .await
    .map_err(manga_record_database("更新漫画提示词布局失败"))?;
    Ok(manga_page_prompt_record_from_model(updated))
}

fn manga_pipeline_record_error(error: MangaPipelineDbError) -> MangaPromptRecordError {
    MangaPromptRecordError::InvalidArgument(error.to_string())
}

pub async fn update_manga_page_prompt_image_prompt(
    db: &impl crate::DbExecutor,
    page_prompt: MangaPagePromptRecord,
    image_prompt: String,
) -> Result<MangaPagePromptRecord, MangaPromptRecordError> {
    require_positive_manga_id(page_prompt.id, "prompt_id")?;
    if page_prompt.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err(MangaPromptRecordError::InvalidArgument(
            "只能修改 page_generation 的图片提示词".to_string(),
        ));
    }
    require_manga_text(&image_prompt, "image_prompt")?;
    let updated = chapter_manga_prompt::ActiveModel {
        id: Set(page_prompt.id),
        image_prompt: Set(image_prompt),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(db)
    .await
    .map_err(manga_record_database("更新漫画图片提示词失败"))?;
    Ok(manga_page_prompt_record_from_model(updated))
}

pub async fn mark_manga_page_prompt_image_completed_with_image(
    db: &impl crate::DbExecutor,
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
