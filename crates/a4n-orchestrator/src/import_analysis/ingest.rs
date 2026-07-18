use a4n_core::models::project::import_review_queue;

use super::{ParsedChapterInput, StartImportAnalysisInput};
use crate::{OrchestratorContext, import_review::ImportReviewCreateInput};

#[derive(Debug)]
struct ValidatedParsedChapter<'a> {
    number: i32,
    title: String,
    content: &'a str,
    content_hash: Option<&'a str>,
}

pub async fn create_import_analysis_reviews(
    context: &OrchestratorContext,
    project_id: i64,
    input: StartImportAnalysisInput,
) -> Result<Vec<import_review_queue::Model>, String> {
    if input.chapters.is_empty() {
        return Err("未解析到可导入章节".to_string());
    }
    validate_import_analysis_stats(&input)?;
    let validated_chapters = input
        .chapters
        .iter()
        .map(validate_parsed_chapter_input)
        .collect::<Result<Vec<_>, _>>()?;

    let parsed_chapters =
        i32::try_from(input.chapters.len()).map_err(|_| "导入章节数量超出 i32 范围".to_string())?;
    let reviews = validated_chapters
        .into_iter()
        .map(|validated| ImportReviewCreateInput {
            project_id,
            filename: input.filename.clone(),
            file_type: input.file_type.clone(),
            detected_encoding: input.detected_encoding.clone(),
            parsed_chapters,
            dedup_paragraphs: input.dedup_paragraphs,
            dedup_rate: input.dedup_rate,
            analysis_result: serde_json::json!({
            "chapter_number": validated.number,
            "chapter_title": validated.title,
            "content": validated.content,
            "content_preview": preview_chars(validated.content, 500),
            "content_hash": validated.content_hash,
            }),
        })
        .collect::<Vec<_>>();

    context
        .create_import_reviews(reviews)
        .await
        .map_err(|err| err.to_string())
}

fn validate_parsed_chapter_input(
    chapter: &ParsedChapterInput,
) -> Result<ValidatedParsedChapter<'_>, String> {
    if chapter.number <= 0 {
        return Err(format!("导入章节序号必须大于 0，收到: {}", chapter.number));
    }

    let title = chapter.title.trim();
    if title.is_empty() {
        return Err(format!(
            "第{}章缺少章节标题，无法开始导入审核",
            chapter.number
        ));
    }

    if chapter.content.trim().is_empty() {
        return Err(format!("第{}章正文为空，无法开始导入审核", chapter.number));
    }

    Ok(ValidatedParsedChapter {
        number: chapter.number,
        title: title.to_string(),
        content: &chapter.content,
        content_hash: chapter.content_hash.as_deref(),
    })
}

fn validate_import_analysis_stats(input: &StartImportAnalysisInput) -> Result<(), String> {
    if input.dedup_paragraphs < 0 {
        return Err(format!(
            "导入去重段落数不能为负数: {}",
            input.dedup_paragraphs
        ));
    }
    if let Some(rate) = input.dedup_rate
        && (!rate.is_finite() || !(0.0..=1.0).contains(&rate))
    {
        return Err(format!("导入去重率必须在 0 到 1 之间: {rate}"));
    }
    Ok(())
}

fn preview_chars(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests;
