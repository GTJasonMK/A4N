use a4n_api_contract::{chapter::ChapterSummary, collaboration::ProjectAccessRole};
use a4n_db::DbHandle;
use a4n_db_models::models::{chapter::chapter, generation::workflow_run};
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use dashmap::DashMap;
use regex::Regex;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use super::OrchestratorContext;
use crate::chapter_content_edit_error_to_app_error;

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

    async fn process_chapter(&self, chapter: chapter::Model) -> Result<(), AppError> {
        let style_tags = analyze_style_tags(&chapter.content);
        a4n_db::chapter_content::update_chapter_style_tags_record(
            self.database.connection(),
            chapter,
            &style_tags,
        )
        .await
        .map_err(chapter_content_edit_error_to_app_error)?;
        Ok(())
    }

    async fn chapters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::chapter::chapter::Model>, AppError> {
        a4n_db::chapter_content::chapters_for_project(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn begin_existing_chapter_content_sync<F>(
        &self,
        chapter_id: i64,
        validate_artifacts: F,
    ) -> Result<workflow_run::Model, AppError>
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
                .begin_chapter_content_sync_in_transaction(&tx, &chapter, &validate_artifacts)
                .await?;
            Ok(run)
        }
        .await;
        finish_chapter_content_transaction(tx, committed, "登记章节资料同步").await
    }

    async fn begin_chapter_content_sync_in_transaction<F>(
        &self,
        tx: &DatabaseTransaction,
        chapter: &chapter::Model,
        validate_artifacts: &F,
    ) -> Result<workflow_run::Model, AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        a4n_chapter_content_core::begin_chapter_content_sync_in_transaction(
            tx,
            chapter,
            validate_artifacts,
        )
        .await
    }

    async fn chapter_by_id_for_analysis(
        &self,
        chapter_id: i64,
    ) -> Result<chapter::Model, AppError> {
        a4n_db::chapter_content::chapter_by_id_for_analysis(self.database.connection(), chapter_id)
            .await
            .map_err(chapter_content_edit_error_to_app_error)
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

fn analyze_style_tags(content: &str) -> Vec<String> {
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

    style_tags
}

impl OrchestratorContext {
    pub(crate) async fn list_chapter_summaries_for_user(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let chapters = self.chapters_for_project(project_id).await?;
        super::chapter_content_projection::chapter_summaries_with_sync_state(self, &chapters).await
    }

    pub(crate) async fn chapter_summary_for_user(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError> {
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
        self.verify_project_permission(chapter.project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        super::chapter_content_projection::chapter_summary_with_sync_state(self, &chapter).await
    }

    pub(crate) async fn process_chapter(&self, chapter: chapter::Model) -> Result<(), AppError> {
        self.chapter_content.process_chapter(chapter).await
    }

    pub(crate) async fn chapters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::chapter::chapter::Model>, AppError> {
        self.chapter_content.chapters_for_project(project_id).await
    }

    pub(crate) fn chapter_content_sync_lock(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Arc<AsyncMutex<()>> {
        self.chapter_content.sync_lock(project_id, chapter_number)
    }

    pub(crate) async fn begin_existing_chapter_content_sync<F>(
        &self,
        chapter_id: i64,
        validate_artifacts: F,
    ) -> Result<workflow_run::Model, AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        self.chapter_content
            .begin_existing_chapter_content_sync(chapter_id, validate_artifacts)
            .await
    }

    pub(crate) async fn begin_chapter_content_sync_in_transaction<F>(
        &self,
        tx: &DatabaseTransaction,
        chapter: &chapter::Model,
        validate_artifacts: &F,
    ) -> Result<workflow_run::Model, AppError>
    where
        F: Fn(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        self.chapter_content
            .begin_chapter_content_sync_in_transaction(tx, chapter, validate_artifacts)
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
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter::ChapterContentReadApplicationPort for OrchestratorContext {
    async fn list(&self, user_id: i64, project_id: i64) -> Result<Vec<ChapterSummary>, AppError> {
        self.list_chapter_summaries_for_user(user_id, project_id)
            .await
    }

    async fn summary(&self, user_id: i64, chapter_id: i64) -> Result<ChapterSummary, AppError> {
        self.chapter_summary_for_user(user_id, chapter_id).await
    }
}

#[cfg(test)]
mod post_process_tests {
    use super::*;

    #[test]
    fn analyzes_style_tags() {
        assert!(analyze_style_tags("纯叙事正文").contains(&"叙事为主".to_string()));
    }
}
