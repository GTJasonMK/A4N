use a4n_core::error::AppError;
use a4n_core::models::{chapter::chapter, generation::workflow_run};
use a4n_db::DbHandle;
use chrono::Utc;
use dashmap::DashMap;
use regex::Regex;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use super::OrchestratorContext;
use crate::chapter_content::{
    CHAPTER_CONTENT_SYNC_CLEANUP_STEP, CHAPTER_CONTENT_SYNC_WORKFLOW_KIND, PostProcessResult,
    chapter_content_edit_error_to_app_error, chapter_content_import_error_to_app_error,
    chapter_content_sync_checkpoint_artifacts, chapter_content_sync_checkpoint_input,
    chapter_content_sync_workflow_business_key, chapter_content_sync_workflow_run_input,
};
use crate::project::{ChapterDerivedCleanupArtifacts, cleanup_chapter_derived_data_in};
use crate::workflow::{workflow_read_app_error, workflow_write_app_error};
use a4n_db::chapter_content::{EditChapterContentInput, ImportChapterContentInput};

/// 章节内容领域私有 owner；生成服务可持有克隆以执行生命周期写路径。
#[derive(Clone)]
pub(crate) struct ChapterContentApplication {
    database: DbHandle,
    sync_locks: Arc<DashMap<(i64, i32), Arc<AsyncMutex<()>>>>,
}

impl ChapterContentApplication {
    pub(crate) fn new(database: DbHandle) -> Self {
        Self {
            database,
            sync_locks: Arc::new(DashMap::new()),
        }
    }

    fn sync_lock(&self, project_id: i64, chapter_number: i32) -> Arc<AsyncMutex<()>> {
        self.sync_locks
            .entry((project_id, chapter_number))
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone()
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

    async fn import_chapter_content_with_sync_plan<F>(
        &self,
        project_id: i64,
        chapter_number: i32,
        content: String,
        title: Option<String>,
        target_generating_operation: &str,
        validate_artifacts: F,
    ) -> Result<(chapter::Model, workflow_run::Model), AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        let word_count = i32::try_from(content.chars().count()).map_err(|_| {
            AppError::InvalidArgument("导入章节正文字符数超出 i32 范围".to_string())
        })?;
        let normalized_title = title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let committed = async {
            let chapter = a4n_db::chapter_content::import_chapter_content_record(
                &tx,
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
            let run = self
                .begin_chapter_content_sync_in_transaction(
                    &tx,
                    &chapter,
                    true,
                    true,
                    &validate_artifacts,
                )
                .await?;
            Ok((chapter, run))
        }
        .await;
        finish_chapter_content_transaction(tx, committed, "导入章节正文并登记资料同步").await
    }

    async fn save_chapter_content_edit_with_sync_plan<F>(
        &self,
        chapter_id: i64,
        content: String,
        title: Option<String>,
        validate_artifacts: F,
    ) -> Result<(chapter::Model, workflow_run::Model), AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let committed = async {
            let chapter = a4n_db::chapter_content::save_chapter_content_edit_record(
                &tx,
                EditChapterContentInput {
                    chapter_id,
                    content,
                    title,
                },
            )
            .await
            .map_err(chapter_content_edit_error_to_app_error)?;
            let run = self
                .begin_chapter_content_sync_in_transaction(
                    &tx,
                    &chapter,
                    true,
                    true,
                    &validate_artifacts,
                )
                .await?;
            Ok((chapter, run))
        }
        .await;
        finish_chapter_content_transaction(tx, committed, "保存章节正文并登记资料同步").await
    }

    async fn begin_existing_chapter_content_sync<F>(
        &self,
        chapter_id: i64,
        cleanup_foreshadowing: bool,
        sync_current_chapter_rag: bool,
        validate_artifacts: F,
    ) -> Result<(chapter::Model, workflow_run::Model), AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let committed = async {
            let chapter = a4n_db::chapter_content::chapter_by_id_for_analysis(&tx, chapter_id)
                .await
                .map_err(chapter_content_edit_error_to_app_error)?;
            let run = self
                .begin_chapter_content_sync_in_transaction(
                    &tx,
                    &chapter,
                    cleanup_foreshadowing,
                    sync_current_chapter_rag,
                    &validate_artifacts,
                )
                .await?;
            Ok((chapter, run))
        }
        .await;
        finish_chapter_content_transaction(tx, committed, "登记章节资料同步").await
    }

    async fn begin_chapter_content_sync_in_transaction<F>(
        &self,
        tx: &DatabaseTransaction,
        chapter: &chapter::Model,
        cleanup_foreshadowing: bool,
        sync_current_chapter_rag: bool,
        validate_artifacts: &F,
    ) -> Result<workflow_run::Model, AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        let business_key =
            chapter_content_sync_workflow_business_key(chapter.project_id, chapter.id);
        let previous =
            a4n_db::workflow::latest_workflow_run_by_business_key_for_update(tx, &business_key)
                .await
                .map_err(workflow_read_app_error)?;
        let mut artifacts = cleanup_chapter_derived_data_in(
            tx,
            chapter.project_id,
            chapter.id,
            chapter.chapter_number,
            cleanup_foreshadowing,
            false,
            false,
        )
        .await?;
        if let Some(previous) = previous.filter(|run| {
            run.workflow_kind == CHAPTER_CONTENT_SYNC_WORKFLOW_KIND
                && a4n_db::workflow::workflow_run_is_running_or_resumable(
                    &run.status,
                    run.can_resume,
                )
        }) {
            let checkpoint = a4n_db::workflow::workflow_checkpoint_model_by_step_in(
                tx,
                previous.id,
                CHAPTER_CONTENT_SYNC_CLEANUP_STEP,
            )
            .await
            .map_err(workflow_read_app_error)?
            .ok_or_else(|| AppError::InvalidState {
                current: format!("chapter_content_sync_run_id={}", previous.id),
                operation: "沿用尚未完成的章节资料清理计划".to_string(),
            })?;
            artifacts.merge(
                chapter_content_sync_checkpoint_artifacts(&previous, &checkpoint).map_err(
                    |operation| AppError::InvalidState {
                        current: format!("chapter_content_sync_run_id={}", previous.id),
                        operation,
                    },
                )?,
            );
        }
        validate_artifacts(&artifacts)?;

        let input = chapter_content_sync_workflow_run_input(chapter, sync_current_chapter_rag);
        let run_fingerprint_json =
            input
                .input_fingerprint_json
                .clone()
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_chapter_content_sync_fingerprint".to_string(),
                    operation: "登记章节资料同步".to_string(),
                })?;
        let run = a4n_db::workflow::upsert_workflow_run_in_transaction(tx, input)
            .await
            .map_err(workflow_write_app_error)?;
        let checkpoint = chapter_content_sync_checkpoint_input(
            run.id,
            run_fingerprint_json,
            chapter,
            &artifacts,
        )?;
        a4n_db::workflow::upsert_workflow_checkpoint_in_transaction(tx, checkpoint)
            .await
            .map_err(workflow_write_app_error)?;
        Ok(run)
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
        cleanup_foreshadowing: bool,
        cleanup_generated_media: bool,
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
            cleanup_foreshadowing,
            cleanup_generated_media,
            false,
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
}

async fn finish_chapter_content_transaction<T>(
    tx: DatabaseTransaction,
    result: Result<T, AppError>,
    operation: &str,
) -> Result<T, AppError> {
    match result {
        Ok(value) => {
            tx.commit().await.map_err(AppError::Database)?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback().await.map_err(|rollback_error| {
                AppError::Internal(anyhow::anyhow!(
                    "{operation}失败：{error}；事务回滚失败：{rollback_error}"
                ))
            })?;
            Err(error)
        }
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

    pub(crate) fn chapter_content_sync_lock(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Arc<AsyncMutex<()>> {
        self.chapter_content.sync_lock(project_id, chapter_number)
    }

    pub(crate) async fn import_chapter_content_with_sync_plan<F>(
        &self,
        project_id: i64,
        chapter_number: i32,
        content: String,
        title: Option<String>,
        target_generating_operation: &str,
        validate_artifacts: F,
    ) -> Result<(chapter::Model, workflow_run::Model), AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        self.chapter_content
            .import_chapter_content_with_sync_plan(
                project_id,
                chapter_number,
                content,
                title,
                target_generating_operation,
                validate_artifacts,
            )
            .await
    }

    pub(crate) async fn save_chapter_content_edit_with_sync_plan<F>(
        &self,
        chapter_id: i64,
        content: String,
        title: Option<String>,
        validate_artifacts: F,
    ) -> Result<(chapter::Model, workflow_run::Model), AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        self.chapter_content
            .save_chapter_content_edit_with_sync_plan(
                chapter_id,
                content,
                title,
                validate_artifacts,
            )
            .await
    }

    pub(crate) async fn begin_existing_chapter_content_sync<F>(
        &self,
        chapter_id: i64,
        cleanup_foreshadowing: bool,
        sync_current_chapter_rag: bool,
        validate_artifacts: F,
    ) -> Result<(chapter::Model, workflow_run::Model), AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        self.chapter_content
            .begin_existing_chapter_content_sync(
                chapter_id,
                cleanup_foreshadowing,
                sync_current_chapter_rag,
                validate_artifacts,
            )
            .await
    }

    pub(crate) async fn begin_chapter_content_sync_in_transaction<F>(
        &self,
        tx: &DatabaseTransaction,
        chapter: &chapter::Model,
        cleanup_foreshadowing: bool,
        sync_current_chapter_rag: bool,
        validate_artifacts: &F,
    ) -> Result<workflow_run::Model, AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        self.chapter_content
            .begin_chapter_content_sync_in_transaction(
                tx,
                chapter,
                cleanup_foreshadowing,
                sync_current_chapter_rag,
                validate_artifacts,
            )
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
        cleanup_foreshadowing: bool,
        cleanup_generated_media: bool,
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
                cleanup_foreshadowing,
                cleanup_generated_media,
                validate_artifacts,
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
