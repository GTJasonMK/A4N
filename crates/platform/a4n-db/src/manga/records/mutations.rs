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
    QuerySelect, Set, sea_query::Expr,
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
    update_manga_page_prompt_layout_expected(db, page_prompt, layout, None).await
}

pub async fn update_manga_page_prompt_layout_expected<C: ConnectionTrait>(
    db: &C,
    page_prompt: MangaPagePromptRecord,
    layout: Value,
    expected_updated_at: Option<&str>,
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
    let updated_at = Utc::now().naive_utc();
    let updated = if let Some(expected_updated_at) = expected_updated_at {
        let expected_updated_at = chrono::NaiveDateTime::parse_from_str(
            expected_updated_at.trim(),
            "%Y-%m-%d %H:%M:%S%.f",
        )
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(expected_updated_at.trim(), "%Y-%m-%d %H:%M:%S")
        })
        .map_err(|_| {
            MangaPromptRecordError::InvalidArgument(
                "漫画提示词版本格式无效，请刷新后重试".to_string(),
            )
        })?;
        let result = chapter_manga_prompt::Entity::update_many()
            .col_expr(
                chapter_manga_prompt::Column::ImagePrompt,
                Expr::value(image_prompt),
            )
            .col_expr(
                chapter_manga_prompt::Column::Layout,
                Expr::value(Some(layout)),
            )
            .col_expr(
                chapter_manga_prompt::Column::UpdatedAt,
                Expr::value(updated_at),
            )
            .filter(chapter_manga_prompt::Column::Id.eq(page_prompt.id))
            .filter(chapter_manga_prompt::Column::UpdatedAt.eq(expected_updated_at))
            .exec(db)
            .await
            .map_err(manga_record_database("更新漫画提示词布局失败"))?;
        if result.rows_affected != 1 {
            return Err(MangaPromptRecordError::InvalidArgument(
                "漫画提示词已被其他操作修改，请刷新后重试".to_string(),
            ));
        }
        chapter_manga_prompt::Entity::find_by_id(page_prompt.id)
            .one(db)
            .await
            .map_err(manga_record_database("读取漫画提示词布局失败"))?
            .ok_or_else(|| MangaPromptRecordError::NotFound("漫画提示词不存在".into()))?
    } else {
        chapter_manga_prompt::ActiveModel {
            id: Set(page_prompt.id),
            image_prompt: Set(image_prompt),
            layout: Set(Some(layout)),
            updated_at: Set(updated_at),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(manga_record_database("更新漫画提示词布局失败"))?
    };
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
    update_manga_page_prompt_image_prompt_expected(db, page_prompt, image_prompt, None).await
}

pub async fn update_manga_page_prompt_image_prompt_expected(
    db: &impl crate::DbExecutor,
    page_prompt: MangaPagePromptRecord,
    image_prompt: String,
    expected_updated_at: Option<&str>,
) -> Result<MangaPagePromptRecord, MangaPromptRecordError> {
    require_positive_manga_id(page_prompt.id, "prompt_id")?;
    if page_prompt.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err(MangaPromptRecordError::InvalidArgument(
            "只能修改 page_generation 的图片提示词".to_string(),
        ));
    }
    require_manga_text(&image_prompt, "image_prompt")?;
    let updated_at = Utc::now().naive_utc();
    let updated = if let Some(expected_updated_at) = expected_updated_at {
        let expected_updated_at = chrono::NaiveDateTime::parse_from_str(
            expected_updated_at.trim(),
            "%Y-%m-%d %H:%M:%S%.f",
        )
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(expected_updated_at.trim(), "%Y-%m-%d %H:%M:%S")
        })
        .map_err(|_| {
            MangaPromptRecordError::InvalidArgument(
                "漫画提示词版本格式无效，请刷新后重试".to_string(),
            )
        })?;
        let result = chapter_manga_prompt::Entity::update_many()
            .col_expr(
                chapter_manga_prompt::Column::ImagePrompt,
                Expr::value(image_prompt),
            )
            .col_expr(
                chapter_manga_prompt::Column::UpdatedAt,
                Expr::value(updated_at),
            )
            .filter(chapter_manga_prompt::Column::Id.eq(page_prompt.id))
            .filter(chapter_manga_prompt::Column::UpdatedAt.eq(expected_updated_at))
            .exec(db)
            .await
            .map_err(manga_record_database("更新漫画图片提示词失败"))?;
        if result.rows_affected != 1 {
            return Err(MangaPromptRecordError::InvalidArgument(
                "漫画提示词已被其他操作修改，请刷新后重试".to_string(),
            ));
        }
        chapter_manga_prompt::Entity::find_by_id(page_prompt.id)
            .one(db)
            .await
            .map_err(manga_record_database("读取漫画图片提示词失败"))?
            .ok_or_else(|| MangaPromptRecordError::NotFound("漫画提示词不存在".into()))?
    } else {
        chapter_manga_prompt::ActiveModel {
            id: Set(page_prompt.id),
            image_prompt: Set(image_prompt),
            updated_at: Set(updated_at),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(manga_record_database("更新漫画图片提示词失败"))?
    };
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
    let updated = chapter_manga_prompt::Entity::update_many()
        .col_expr(
            chapter_manga_prompt::Column::GeneratedImageId,
            Expr::value(image_id),
        )
        .col_expr(
            chapter_manga_prompt::Column::Status,
            Expr::value("completed"),
        )
        .col_expr(
            chapter_manga_prompt::Column::UpdatedAt,
            Expr::value(updated_at),
        )
        .filter(chapter_manga_prompt::Column::Id.eq(page_prompt_id))
        .filter(chapter_manga_prompt::Column::GeneratedImageId.is_null())
        .exec(db)
        .await
        .map_err(manga_record_database("回写漫画提示词生成图片失败"))?;
    if updated.rows_affected == 0 {
        let existing = chapter_manga_prompt::Entity::find_by_id(page_prompt_id)
            .one(db)
            .await
            .map_err(manga_record_database("读取漫画提示词图片状态失败"))?
            .ok_or_else(|| MangaPromptRecordError::NotFound("漫画提示词不存在".into()))?;
        if existing.generated_image_id == Some(image_id) {
            return Ok(());
        }
        if existing.generated_image_id.is_some() {
            return Err(MangaPromptRecordError::Conflict(
                "漫画页面已有成图，请先删除后再生成".into(),
            ));
        }
        return Err(MangaPromptRecordError::Conflict(
            "漫画页面图片状态更新冲突，请刷新后重试".into(),
        ));
    }
    Ok(())
}
