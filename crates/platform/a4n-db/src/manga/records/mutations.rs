use crate::generation::delete_generation_run_targets_for;
use crate::manga::{
    MangaPipelineDbError, MangaPromptRecordError, MangaPromptUpdatedAt, manga_record_database,
    manga_video_error, require_manga_text, require_positive_manga_id,
    validate_manga_page_prompt_layout,
};
use a4n_db_models::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use a4n_db_models::models::{chapter::manga_pipeline_artifact, project::novel_project};
use a4n_domain::manga::MangaPagePromptRecord;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
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
    lock_manga_project(&txn, project_id).await?;
    let deleted = delete_manga_prompt_outputs(&txn, project_id, chapter_id, None).await?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画提示词删除事务失败"))?;
    Ok(deleted)
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
    lock_manga_project(&txn, project_id).await?;
    delete_manga_prompt_outputs(&txn, project_id, chapter_id, Some(page_number)).await?;
    txn.commit()
        .await
        .map_err(manga_record_database("提交漫画页面删除事务失败"))
}

async fn lock_manga_project<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<(), MangaPromptRecordError> {
    novel_project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(manga_record_database("锁定漫画项目删除范围失败"))?
        .ok_or_else(|| MangaPromptRecordError::NotFound("漫画项目不存在".into()))?;
    Ok(())
}

async fn delete_manga_prompt_outputs<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    page_number: Option<i32>,
) -> Result<u64, MangaPromptRecordError> {
    let mut query = chapter_manga_prompt::Entity::find()
        .select_only()
        .column(chapter_manga_prompt::Column::Id)
        .column(chapter_manga_prompt::Column::PageNumber)
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id);
    if let Some(page_number) = page_number {
        query = query.filter(chapter_manga_prompt::Column::PageNumber.eq(page_number));
    }
    let rows = query
        .into_tuple::<(i64, i32)>()
        .all(db)
        .await
        .map_err(manga_record_database("读取待删除漫画页面失败"))?;
    let prompt_ids = rows.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    delete_generation_run_targets_for(db, "chapter_manga_prompt", &prompt_ids).await?;

    let image_paths = match page_number {
        None => {
            crate::video::delete_manga_chapter_image_records_in(db, project_id, chapter_id, None)
                .await
                .map_err(manga_video_error)?
        }
        Some(page_number) => crate::video::delete_manga_page_image_records_in(
            db,
            project_id,
            chapter_id,
            page_number,
            None,
        )
        .await
        .map_err(manga_video_error)?,
    };

    let mut delete_prompts = chapter_manga_prompt::Entity::delete_many()
        .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id));
    let mut delete_artifacts = manga_pipeline_artifact::Entity::delete_many()
        .filter(manga_pipeline_artifact::Column::ProjectId.eq(project_id))
        .filter(manga_pipeline_artifact::Column::ChapterId.eq(chapter_id));
    if let Some(page_number) = page_number {
        delete_prompts =
            delete_prompts.filter(chapter_manga_prompt::Column::PageNumber.eq(page_number));
        delete_artifacts =
            delete_artifacts.filter(manga_pipeline_artifact::Column::PageNumber.eq(page_number));
    }
    let deleted = delete_prompts
        .exec(db)
        .await
        .map_err(manga_record_database("删除漫画页面提示词失败"))?;
    delete_artifacts
        .exec(db)
        .await
        .map_err(manga_record_database("删除漫画页面阶段产物失败"))?;
    if !image_paths.is_empty() {
        let fingerprint = crate::generation::canonical_json_sha256(&(&prompt_ids, &image_paths))
            .map_err(|error| MangaPromptRecordError::InvalidArgument(error.to_string()))?;
        crate::media::schedule_unreferenced_media_cleanup(
            db,
            project_id,
            &image_paths,
            format!("manga-page-delete:{project_id}:{chapter_id}:{fingerprint}"),
        )
        .await
        .map_err(manga_record_database("安排漫画页成图文件清理失败"))?;
    }
    Ok(deleted.rows_affected)
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
