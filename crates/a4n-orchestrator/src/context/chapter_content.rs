use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_db::DbHandle;
use chrono::Utc;
use regex::Regex;
use sea_orm::TransactionTrait;

use super::OrchestratorContext;
use crate::chapter_content::{
    PostProcessResult, chapter_content_edit_error_to_app_error,
    chapter_content_import_error_to_app_error, ensure_chapter_content_not_empty,
    ensure_chapter_not_generating, ensure_import_chapter_number,
};
use crate::project::{
    ChapterDerivedCleanupArtifacts, ChapterDerivedCleanupResult, cleanup_chapter_derived_data_in,
    delete_chapter_content_record_in,
};
use a4n_db::chapter_content::{EditChapterContentInput, ImportChapterContentInput};

/// 章节内容领域私有 owner；生成服务可持有克隆以执行生命周期写路径。
#[derive(Clone)]
pub(crate) struct ChapterContentApplication {
    database: DbHandle,
}

impl ChapterContentApplication {
    pub(crate) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn process_chapter(&self, chapter_id: i64) -> Result<PostProcessResult, AppError> {
        let chapter = a4n_db::chapter_content::chapter_for_post_process(
            self.database.connection(),
            chapter_id,
        )
        .await
        .map_err(chapter_content_edit_error_to_app_error)?;
        let result = analyze_content(&chapter.content);
        a4n_db::chapter_content::update_chapter_style_tags_record(
            self.database.connection(),
            chapter_id,
            &result.style_tags,
            Utc::now().naive_utc(),
        )
        .await
        .map_err(chapter_content_edit_error_to_app_error)?;
        Ok(result)
    }

    async fn chapters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::chapter::chapter::Model>, AppError> {
        a4n_db::chapter_content::chapters_for_project(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    /// 校验章节号与正文后导入章节（含 prepare + 落库）。
    async fn import_chapter_content_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        content: String,
        title: Option<String>,
        target_generating_operation: &str,
    ) -> Result<(a4n_core::models::chapter::chapter::Model, i32), AppError> {
        ensure_import_chapter_number(chapter_number)?;
        ensure_chapter_content_not_empty(&content)?;
        let word_count = i32::try_from(content.chars().count()).map_err(|_| {
            AppError::InvalidArgument("导入章节正文字符数超出 i32 范围".to_string())
        })?;
        let normalized_title = title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let chapter = a4n_db::chapter_content::import_chapter_content_record(
            self.database.connection(),
            ImportChapterContentInput {
                project_id,
                chapter_number,
                content,
                normalized_title,
                word_count,
                now: Utc::now().naive_utc(),
            },
            target_generating_operation,
        )
        .await
        .map_err(chapter_content_import_error_to_app_error)?;
        Ok((chapter, word_count))
    }

    /// 校验生成中门禁与正文非空后保存章节编辑。
    async fn save_chapter_content_edit_validated(
        &self,
        chapter_id: i64,
        content: String,
        title: Option<String>,
    ) -> Result<a4n_core::models::chapter::chapter::Model, AppError> {
        let chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        ensure_chapter_not_generating(&chapter, "更新章节正文")?;
        ensure_chapter_content_not_empty(&content)?;
        a4n_db::chapter_content::save_chapter_content_edit_record(
            self.database.connection(),
            EditChapterContentInput {
                chapter_id,
                content,
                title,
            },
        )
        .await
        .map_err(chapter_content_edit_error_to_app_error)
    }

    async fn chapter_by_id_for_analysis(
        &self,
        chapter_id: i64,
    ) -> Result<chapter::Model, AppError> {
        a4n_db::chapter_content::chapter_by_id_for_analysis(self.database.connection(), chapter_id)
            .await
            .map_err(chapter_content_edit_error_to_app_error)
    }

    /// 章节派生数据清理事务：校验失败时回滚，成功后提交。
    async fn cleanup_chapter_derived_data_record<F>(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        include_generated_outputs: bool,
        validate_artifacts: F,
    ) -> Result<ChapterDerivedCleanupArtifacts, AppError>
    where
        F: FnOnce(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let artifacts = cleanup_chapter_derived_data_in(
            &tx,
            project_id,
            chapter_id,
            chapter_number,
            include_generated_outputs,
        )
        .await?;
        if let Err(error) = validate_artifacts(&artifacts) {
            if let Err(rollback_error) = tx.rollback().await {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "章节派生数据清理校验失败：{error}；事务回滚失败：{rollback_error}"
                )));
            }
            return Err(error);
        }
        tx.commit().await.map_err(AppError::Database)?;
        Ok(artifacts)
    }

    /// 删除章节内容/大纲事务：校验失败时回滚，成功后提交。
    async fn delete_chapter_content_records_and_outlines_record<F>(
        &self,
        project_id: i64,
        chapters: Vec<chapter::Model>,
        outline_ids: &[i64],
        validate_results: F,
    ) -> Result<Vec<ChapterDerivedCleanupResult>, AppError>
    where
        F: FnOnce(&[ChapterDerivedCleanupResult]) -> Result<(), AppError>,
    {
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let mut cleanup_results = Vec::with_capacity(chapters.len());
        for chapter in chapters {
            let artifacts = delete_chapter_content_record_in(
                &tx,
                project_id,
                chapter.id,
                chapter.chapter_number,
            )
            .await?;
            cleanup_results.push(ChapterDerivedCleanupResult {
                chapter_id: chapter.id,
                chapter_number: chapter.chapter_number,
                artifacts,
            });
        }
        if let Err(error) = validate_results(&cleanup_results) {
            if let Err(rollback_error) = tx.rollback().await {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "章节内容清理校验失败：{error}；事务回滚失败：{rollback_error}"
                )));
            }
            return Err(error);
        }
        a4n_db::project::delete_chapter_outlines_by_ids(&tx, outline_ids)
            .await
            .map_err(crate::errors::project_cleanup_error_to_app_error)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(cleanup_results)
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

impl OrchestratorContext {
    pub(crate) async fn process_chapter(
        &self,
        chapter_id: i64,
    ) -> Result<PostProcessResult, AppError> {
        self.chapter_content.process_chapter(chapter_id).await
    }

    pub async fn chapters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::chapter::chapter::Model>, AppError> {
        self.chapter_content.chapters_for_project(project_id).await
    }

    /// 导入章节正文：章节号/非空门禁 + prepare + 落库。
    pub(crate) async fn import_chapter_content_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        content: String,
        title: Option<String>,
        target_generating_operation: &str,
    ) -> Result<(a4n_core::models::chapter::chapter::Model, i32), AppError> {
        self.chapter_content
            .import_chapter_content_validated(
                project_id,
                chapter_number,
                content,
                title,
                target_generating_operation,
            )
            .await
    }

    /// 保存章节正文编辑：生成中门禁 + 正文非空后落库。
    pub(crate) async fn save_chapter_content_edit_validated(
        &self,
        chapter_id: i64,
        content: String,
        title: Option<String>,
    ) -> Result<a4n_core::models::chapter::chapter::Model, AppError> {
        self.chapter_content
            .save_chapter_content_edit_validated(chapter_id, content, title)
            .await
    }

    pub(crate) async fn chapter_by_id_for_analysis(
        &self,
        chapter_id: i64,
    ) -> Result<chapter::Model, AppError> {
        self.chapter_content
            .chapter_by_id_for_analysis(chapter_id)
            .await
    }

    pub async fn cleanup_chapter_derived_data_record<F>(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        include_generated_outputs: bool,
        validate_artifacts: F,
    ) -> Result<ChapterDerivedCleanupArtifacts, AppError>
    where
        F: FnOnce(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        self.chapter_content
            .cleanup_chapter_derived_data_record(
                project_id,
                chapter_id,
                chapter_number,
                include_generated_outputs,
                validate_artifacts,
            )
            .await
    }

    pub async fn delete_chapter_content_records_and_outlines_record<F>(
        &self,
        project_id: i64,
        chapters: Vec<chapter::Model>,
        outline_ids: &[i64],
        validate_results: F,
    ) -> Result<Vec<ChapterDerivedCleanupResult>, AppError>
    where
        F: FnOnce(&[ChapterDerivedCleanupResult]) -> Result<(), AppError>,
    {
        self.chapter_content
            .delete_chapter_content_records_and_outlines_record(
                project_id,
                chapters,
                outline_ids,
                validate_results,
            )
            .await
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
