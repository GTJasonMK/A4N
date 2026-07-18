pub use a4n_db::chapter_content::{
    ChapterContentEditError, ChapterContentImportError, ChapterGenerationExistingRecord,
    ChapterGenerationLifecycleError, EditChapterContentInput, GeneratedChapterContentRecordInput,
    ImportChapterContentInput, ImportChapterContentTarget, StoreChapterAnalysisRecordInput,
    chapter_by_id_for_analysis, chapter_for_post_process, store_chapter_analysis_record,
    update_chapter_style_tags_record,
};

use crate::OrchestratorContext;
use a4n_core::error::AppError;
use chrono::Utc;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct PostProcessResult {
    pub style_tags: Vec<String>,
    pub char_count: usize,
    pub chinese_word_count: usize,
    pub paragraph_count: usize,
    pub dialogue_ratio: f64,
    pub quality_score: f64,
}

impl OrchestratorContext {
    pub async fn process_chapter(&self, chapter_id: i64) -> Result<PostProcessResult, AppError> {
        let chapter =
            a4n_db::chapter_content::chapter_for_post_process(self.db.connection(), chapter_id)
                .await
                .map_err(chapter_content_edit_error)?;
        let result = analyze_content(&chapter.content);
        a4n_db::chapter_content::update_chapter_style_tags_record(
            self.db.connection(),
            chapter_id,
            &result.style_tags,
            Utc::now().naive_utc(),
        )
        .await
        .map_err(chapter_content_edit_error)?;
        Ok(result)
    }

    pub async fn chapters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::chapter::chapter::Model>, AppError> {
        a4n_db::chapter_content::chapters_for_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn prepare_import_chapter_content(
        &self,
        project_id: i64,
        chapter_number: i32,
        target_generating_operation: &str,
    ) -> Result<ImportChapterContentTarget, ChapterContentImportError> {
        a4n_db::chapter_content::prepare_import_chapter_content(
            self.db.connection(),
            project_id,
            chapter_number,
            target_generating_operation,
        )
        .await
    }

    pub async fn import_chapter_content(
        &self,
        input: ImportChapterContentInput,
        target_generating_operation: &str,
    ) -> Result<a4n_core::models::chapter::chapter::Model, ChapterContentImportError> {
        a4n_db::chapter_content::import_chapter_content_record(
            self.db.connection(),
            input,
            target_generating_operation,
        )
        .await
    }

    pub async fn save_chapter_content_edit(
        &self,
        input: EditChapterContentInput,
    ) -> Result<a4n_core::models::chapter::chapter::Model, ChapterContentEditError> {
        a4n_db::chapter_content::save_chapter_content_edit_record(self.db.connection(), input).await
    }
}

fn chapter_content_edit_error(error: ChapterContentEditError) -> AppError {
    match error {
        ChapterContentEditError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ChapterContentEditError::NotFound(message) => AppError::NotFound(message),
        ChapterContentEditError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        ChapterContentEditError::Database(error) => AppError::Database(error),
    }
}

fn analyze_content(content: &str) -> PostProcessResult {
    let char_count = content.chars().count();
    let chinese_re = Regex::new(r"[\u{4e00}-\u{9fff}\u{3400}-\u{4dbf}]").unwrap();
    let chinese_word_count = chinese_re.find_iter(content).count();
    let paragraph_count = content
        .split("\n\n")
        .filter(|paragraph| !paragraph.trim().is_empty())
        .count();
    let dialogue_re = Regex::new(r#"[""][^""""]+[""']|「[^」]+」|『[^』]+』"#).unwrap();
    let dialogue_chars: usize = dialogue_re
        .find_iter(content)
        .map(|matched| matched.as_str().chars().count())
        .sum();
    let dialogue_ratio = if char_count > 0 {
        dialogue_chars as f64 / char_count as f64
    } else {
        0.0
    };

    let mut style_tags = Vec::new();
    if dialogue_ratio > 0.3 {
        style_tags.push("对话密集".to_string());
    } else if dialogue_ratio < 0.05 {
        style_tags.push("叙事为主".to_string());
    }
    let average_paragraph_length = if paragraph_count > 0 {
        chinese_word_count as f64 / paragraph_count as f64
    } else {
        0.0
    };
    if average_paragraph_length > 500.0 {
        style_tags.push("长段叙事".to_string());
    } else if average_paragraph_length < 80.0 && paragraph_count > 3 {
        style_tags.push("碎片化".to_string());
    }

    let mut quality_score: f64 = 70.0;
    if (30.0..=300.0).contains(&average_paragraph_length) {
        quality_score += 10.0;
    }
    if (0.1..=0.5).contains(&dialogue_ratio) {
        quality_score += 10.0;
    }
    if chinese_word_count < 500 {
        quality_score -= 20.0;
    } else if chinese_word_count > 20_000 {
        quality_score -= 5.0;
    }

    PostProcessResult {
        style_tags,
        char_count,
        chinese_word_count,
        paragraph_count,
        dialogue_ratio,
        quality_score: quality_score.clamp(0.0, 100.0),
    }
}

#[cfg(test)]
mod post_process_tests {
    use super::*;

    #[test]
    fn analyzes_chinese_content() {
        let result =
            analyze_content("这是第一章的内容。\n\n包含多个段落。\n\n「这是一段对话」主角说道。");
        assert!(result.chinese_word_count > 0);
        assert_eq!(result.paragraph_count, 3);
        assert!(result.dialogue_ratio > 0.0);
    }

    #[test]
    fn quality_score_stays_in_range() {
        let result = analyze_content(&"测试内容，用于验证质量评分范围。".repeat(50));
        assert!((0.0..=100.0).contains(&result.quality_score));
    }
}
