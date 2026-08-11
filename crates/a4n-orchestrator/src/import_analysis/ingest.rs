use a4n_core::error::AppError;
use a4n_core::models::project::import_review_queue;

use super::ParsedChapterInput;
use crate::{
    OrchestratorContext, collaboration::ProjectAccessRole, import_review::ImportReviewCreateInput,
};

#[derive(Debug)]
struct ValidatedParsedChapter<'a> {
    number: i32,
    title: String,
    content: &'a str,
}

pub async fn create_import_analysis_reviews_for_user_validated(
    context: &OrchestratorContext,
    user_id: i64,
    project_id: i64,
    filename: &str,
    chapters: &[ParsedChapterInput],
) -> Result<Vec<import_review_queue::Model>, AppError> {
    context
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
        .await?;
    let filename = filename.trim();
    if filename.is_empty() {
        return Err(AppError::InvalidArgument(
            "上传导入文件缺少文件名".to_string(),
        ));
    }
    if chapters.is_empty() {
        return Err(AppError::InvalidArgument("未解析到可导入章节".to_string()));
    }
    let validated_chapters = chapters
        .iter()
        .map(validate_parsed_chapter_input)
        .collect::<Result<Vec<_>, _>>()?;

    let reviews = validated_chapters
        .into_iter()
        .map(|validated| ImportReviewCreateInput {
            project_id,
            filename: filename.to_string(),
            analysis_result: serde_json::json!({
                "chapter_number": validated.number,
                "chapter_title": validated.title,
                "content": validated.content,
            }),
        })
        .collect::<Vec<_>>();

    context.create_import_reviews(reviews).await
}

fn validate_parsed_chapter_input(
    chapter: &ParsedChapterInput,
) -> Result<ValidatedParsedChapter<'_>, AppError> {
    if chapter.number <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "导入章节序号必须大于 0，收到: {}",
            chapter.number
        )));
    }

    let title = chapter.title.trim();
    if title.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "第{}章缺少章节标题，无法开始导入审核",
            chapter.number
        )));
    }

    if chapter.content.trim().is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "第{}章正文为空，无法开始导入审核",
            chapter.number
        )));
    }

    Ok(ValidatedParsedChapter {
        number: chapter.number,
        title: title.to_string(),
        content: &chapter.content,
    })
}

#[cfg(test)]
mod tests;
