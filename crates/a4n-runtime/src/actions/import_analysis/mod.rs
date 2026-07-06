mod blueprint;
mod chapter_outline;
mod part_outline;
mod progress;
mod workflow;

use crate::actions::access;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::state::AppState;
use a4n_core::models::project::import_review_queue;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProjectAnalysisSummary {
    pub progress: Value,
    pub analyzed_chapters: usize,
    pub outlined_chapters: usize,
    pub generated_parts: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartImportAnalysisInput {
    pub filename: String,
    pub file_type: String,
    pub detected_encoding: Option<String>,
    pub chapters: Vec<ParsedChapterInput>,
    pub dedup_paragraphs: i32,
    pub dedup_rate: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedChapterInput {
    pub number: i32,
    pub title: String,
    pub content: String,
    pub content_hash: Option<String>,
}

#[derive(Debug)]
struct ValidatedParsedChapter<'a> {
    number: i32,
    title: String,
    content: &'a str,
    content_hash: Option<&'a str>,
}

pub async fn start_import_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: StartImportAnalysisInput,
) -> Result<Vec<import_review_queue::Model>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    if input.chapters.is_empty() {
        return Err("未解析到可导入章节".to_string());
    }
    validate_import_analysis_stats(&input)?;
    let validated_chapters = input
        .chapters
        .iter()
        .map(validate_parsed_chapter_input)
        .collect::<Result<Vec<_>, _>>()?;

    let now = Utc::now().naive_utc();
    let mut reviews = Vec::with_capacity(input.chapters.len());

    for validated in validated_chapters {
        let model = import_review_queue::ActiveModel {
            project_id: Set(project_id),
            filename: Set(input.filename.clone()),
            file_type: Set(input.file_type.clone()),
            detected_encoding: Set(input.detected_encoding.clone()),
            parsed_chapters: Set(input.chapters.len() as i32),
            dedup_paragraphs: Set(input.dedup_paragraphs),
            dedup_rate: Set(input.dedup_rate),
            analysis_result: Set(Some(serde_json::json!({
                "chapter_number": validated.number,
                "chapter_title": validated.title,
                "content": validated.content,
                "content_preview": preview_chars(validated.content, 500),
                "content_hash": validated.content_hash,
            }))),
            review_status: Set("pending".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

        reviews.push(model);
    }

    Ok(reviews)
}

pub async fn preview_import_project_analysis_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let chapters = chapter_outline::importable_chapters(state.db.as_ref(), project_id).await?;
    if chapters.is_empty() {
        return Err("项目没有可用于导入分析的章节正文".to_string());
    }

    let analyzer = a4n_services::chapter::analysis::ChapterAnalysisService::new(state.db.clone());
    let mut steps = Vec::with_capacity(chapters.len() * 4 + 2);
    for chapter in chapters {
        let requests = analyzer
            .build_analysis_requests(chapter.id, state.engine.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        for request in requests {
            steps.push(ChatStatusPreviewStep::llm(
                format!("import_chapter_analysis_{}_{}", chapter.id, request.key),
                format!("第 {} 章{}", chapter.chapter_number, request.title),
                Some(request.prompt_name),
                "ready",
                Some("导入项目分析的分析阶段会按顺序发送该请求".to_string()),
                Some(request.request),
            ));
        }
    }
    steps.push(ChatStatusPreviewStep::llm(
        "import_analysis_character_sync",
        "导入章节角色发现和角色同步请求",
        Some("character_discovery".to_string()),
        "blocked",
        Some("这些请求依赖对应章节结构化分析输出，运行前无法提前构造真实请求".to_string()),
        None,
    ));
    steps.push(ChatStatusPreviewStep::llm(
        "import_analysis_reverse_blueprint",
        "导入蓝图反推请求",
        Some("reverse_blueprint".to_string()),
        "blocked",
        Some("该请求依赖前序章节分析和大纲更新后的上下文，运行前无法提前构造真实请求".to_string()),
        None,
    ));
    chat_status_preview_for_state(
        state,
        "import_project_analysis",
        "导入项目分析请求预览",
        steps,
        "llm_chat",
    )
    .await
}

pub async fn get_project_import_analysis_progress(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Option<Value>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    workflow::get_project_import_analysis_progress(state, project_id).await
}

pub async fn run_import_project_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImportProjectAnalysisSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    workflow::run_import_project_analysis(state, user_id, project_id).await
}

pub async fn cancel_import_analysis(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<u64, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    workflow::cancel_import_analysis(state, project_id).await
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
mod tests {
    use super::*;

    fn parsed_chapter(number: i32, title: &str, content: &str) -> ParsedChapterInput {
        ParsedChapterInput {
            number,
            title: title.to_string(),
            content: content.to_string(),
            content_hash: Some("abc123".to_string()),
        }
    }

    fn start_input(dedup_paragraphs: i32, dedup_rate: Option<f32>) -> StartImportAnalysisInput {
        StartImportAnalysisInput {
            filename: "import.txt".to_string(),
            file_type: "txt".to_string(),
            detected_encoding: Some("utf-8".to_string()),
            chapters: vec![parsed_chapter(1, "旧城回声", "正文")],
            dedup_paragraphs,
            dedup_rate,
        }
    }

    #[test]
    fn validate_import_analysis_stats_rejects_negative_dedup_paragraphs() {
        let input = start_input(-1, Some(0.0));

        let err = validate_import_analysis_stats(&input)
            .expect_err("negative dedup paragraph count must fail");

        assert!(err.contains("去重段落数不能为负数"));
        assert!(err.contains("-1"));
    }

    #[test]
    fn validate_import_analysis_stats_rejects_out_of_range_dedup_rate() {
        let input = start_input(1, Some(1.5));

        let err = validate_import_analysis_stats(&input).expect_err("dedup rate above 1 must fail");

        assert!(err.contains("去重率必须在 0 到 1 之间"));
        assert!(err.contains("1.5"));
    }

    #[test]
    fn validate_import_analysis_stats_accepts_valid_rate_bounds() {
        validate_import_analysis_stats(&start_input(0, Some(0.0)))
            .expect("zero dedup rate should be valid");
        validate_import_analysis_stats(&start_input(1, Some(1.0)))
            .expect("full dedup rate should be valid");
        validate_import_analysis_stats(&start_input(1, None))
            .expect("missing dedup rate should be valid");
    }

    #[test]
    fn validate_parsed_chapter_input_rejects_non_positive_number() {
        let chapter = parsed_chapter(0, "旧城回声", "正文");

        let err = validate_parsed_chapter_input(&chapter)
            .expect_err("non-positive chapter number must fail");

        assert!(err.contains("必须大于 0"));
    }

    #[test]
    fn validate_parsed_chapter_input_rejects_blank_title() {
        let chapter = parsed_chapter(3, " \n\t ", "正文");

        let err = validate_parsed_chapter_input(&chapter).expect_err("blank title must fail");

        assert!(err.contains("缺少章节标题"));
    }

    #[test]
    fn validate_parsed_chapter_input_rejects_blank_content() {
        let chapter = parsed_chapter(3, "旧城回声", " \n\t ");

        let err = validate_parsed_chapter_input(&chapter).expect_err("blank content must fail");

        assert!(err.contains("正文为空"));
    }

    #[test]
    fn validate_parsed_chapter_input_accepts_real_chapter() {
        let chapter = parsed_chapter(3, "  旧城回声  ", "  林岚发现星潮线索。  ");

        let validated = validate_parsed_chapter_input(&chapter).expect("valid chapter should pass");

        assert_eq!(validated.number, 3);
        assert_eq!(validated.title, "旧城回声");
        assert_eq!(validated.content, "  林岚发现星潮线索。  ");
        assert_eq!(validated.content_hash, Some("abc123"));
    }
}
