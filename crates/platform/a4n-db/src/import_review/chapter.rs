use super::contracts::ImportReviewMutationError;
use a4n_db_models::models::{
    chapter::chapter::{self, STATUS_IMPORTED},
    project::import_review_queue,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn json_i32(value: &Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

pub(super) fn required_import_chapter_number(
    analysis: &Value,
    review_id: i64,
) -> Result<i32, ImportReviewMutationError> {
    let chapter_number = json_i32(analysis, "chapter_number").ok_or_else(|| {
        ImportReviewMutationError::InvalidArgument(format!(
            "导入审核缺少章节序号: review_id={review_id}"
        ))
    })?;
    if chapter_number <= 0 {
        return Err(ImportReviewMutationError::InvalidArgument(format!(
            "导入审核章节序号必须为正整数: review_id={review_id}, chapter_number={chapter_number}"
        )));
    }
    Ok(chapter_number)
}

fn required_import_chapter_title(
    analysis: &Value,
    review_id: i64,
) -> Result<String, ImportReviewMutationError> {
    json_string(analysis, "chapter_title").ok_or_else(|| {
        ImportReviewMutationError::InvalidArgument(format!(
            "导入审核缺少章节标题，无法接受。请在审核中补充标题后再接受: review_id={review_id}"
        ))
    })
}

fn ensure_chapter_not_generating(
    chapter: &chapter::Model,
    operation: &str,
) -> Result<(), ImportReviewMutationError> {
    if chapter.status == "generating" {
        return Err(ImportReviewMutationError::InvalidState {
            current: chapter.status.clone(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}

pub(super) async fn upsert_imported_chapter_from_review<C>(
    db: &C,
    review: &import_review_queue::Model,
) -> Result<chapter::Model, ImportReviewMutationError>
where
    C: ConnectionTrait,
{
    let analysis = &review.analysis_result;
    let chapter_number = required_import_chapter_number(analysis, review.id)?;
    let title = required_import_chapter_title(analysis, review.id)?;
    let content = json_string(analysis, "content").ok_or_else(|| {
        ImportReviewMutationError::InvalidArgument(format!(
            "导入审核缺少完整正文，无法接受。请重新导入该文件后再接受: review_id={}",
            review.id
        ))
    })?;
    let word_count = i32::try_from(content.chars().count()).map_err(|_| {
        ImportReviewMutationError::InvalidArgument(format!(
            "导入章节正文字符数超出 i32 范围: review_id={}",
            review.id
        ))
    })?;
    let summary = json_string(analysis, "summary");
    let now = Utc::now().naive_utc();
    let existing = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(review.project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await?;
    if let Some(existing) = existing {
        ensure_chapter_not_generating(&existing, "接受导入审核")?;
        let mut active: chapter::ActiveModel = existing.into();
        active.title = Set(Some(title));
        crate::chapter_content::assign_chapter_content(
            &mut active,
            content,
            word_count,
            STATUS_IMPORTED,
            summary,
            now,
        );
        active.update(db).await.map_err(Into::into)
    } else {
        let mut active = chapter::ActiveModel {
            project_id: Set(review.project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(Some(title)),
            created_at: Set(now),
            ..Default::default()
        };
        crate::chapter_content::assign_chapter_content(
            &mut active,
            content,
            word_count,
            STATUS_IMPORTED,
            summary,
            now,
        );
        active.insert(db).await.map_err(Into::into)
    }
}
