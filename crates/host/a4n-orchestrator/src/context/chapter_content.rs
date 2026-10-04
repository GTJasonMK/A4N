use super::OrchestratorContext;
use crate::chapter_content_edit_error_to_app_error;
use a4n_api_contract::{
    chapter::{ChapterAnalysisEntry, ChapterIndexEntry, ChapterMetadata, ChapterSummary},
    collaboration::ProjectAccessRole,
};
use a4n_db::{DbHandle, transaction::run_in_transaction, workflow::WorkflowRunSummary};
use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use dashmap::DashMap;
use regex::Regex;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

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

    async fn process_chapter(&self, chapter: ChapterRecord) -> Result<(), AppError> {
        let style_tags = analyze_style_tags(&chapter.content);
        a4n_db::chapter_content::update_chapter_style_tags_record(
            self.database.executor(),
            chapter.id,
            &style_tags,
        )
        .await
        .map_err(chapter_content_edit_error_to_app_error)?;
        Ok(())
    }

    async fn chapters_for_project(&self, project_id: i64) -> Result<Vec<ChapterRecord>, AppError> {
        a4n_db::chapter_content::chapters_for_project(self.database.executor(), project_id)
            .await
            .map_err(AppError::from)
    }

    async fn begin_existing_chapter_content_sync(
        &self,
        chapter_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        run_in_transaction(self.database.executor(), "登记章节资料同步", |tx| {
            Box::pin(async move {
                let result = async {
                    let chapter =
                        a4n_db::chapter_content::chapter_by_id_for_analysis(&tx, chapter_id)
                            .await
                            .map_err(chapter_content_edit_error_to_app_error)?;
                    a4n_chapter_content_core::begin_chapter_content_sync_in_transaction(
                        &tx, &chapter,
                    )
                    .await
                }
                .await;
                (tx, result)
            }) as a4n_db::transaction::TransactionWork<'_, WorkflowRunSummary>
        })
        .await
    }

    async fn chapter_by_id_for_analysis(&self, chapter_id: i64) -> Result<ChapterRecord, AppError> {
        a4n_db::chapter_content::chapter_by_id_for_analysis(self.database.executor(), chapter_id)
            .await
            .map_err(chapter_content_edit_error_to_app_error)
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
        self::chapter_summaries_with_sync_state(self, &chapters).await
    }

    pub(crate) async fn chapter_summary_for_user(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError> {
        self.chapter_summary_for_user_in_project(user_id, chapter_id, None)
            .await
    }

    async fn chapter_summary_for_user_in_project(
        &self,
        user_id: i64,
        chapter_id: i64,
        project_id: Option<i64>,
    ) -> Result<ChapterSummary, AppError> {
        if project_id.is_some_and(|id| id <= 0) {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
        if project_id.is_some_and(|id| id != chapter.project_id) {
            return Err(AppError::NotFound(format!(
                "章节不属于指定项目: {chapter_id}"
            )));
        }
        self.verify_project_permission(chapter.project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self::chapter_summary_with_sync_state(self, &chapter).await
    }

    pub(crate) async fn process_chapter(&self, chapter: ChapterRecord) -> Result<(), AppError> {
        self.chapter_content.process_chapter(chapter).await
    }

    pub(crate) async fn chapters_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<ChapterRecord>, AppError> {
        self.chapter_content.chapters_for_project(project_id).await
    }

    pub(crate) fn chapter_content_sync_lock(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Arc<AsyncMutex<()>> {
        self.chapter_content.sync_lock(project_id, chapter_number)
    }

    pub(crate) async fn begin_existing_chapter_content_sync(
        &self,
        chapter_id: i64,
    ) -> Result<WorkflowRunSummary, AppError> {
        self.chapter_content
            .begin_existing_chapter_content_sync(chapter_id)
            .await
    }

    pub(crate) async fn chapter_by_id_for_analysis(
        &self,
        chapter_id: i64,
    ) -> Result<ChapterRecord, AppError> {
        self.chapter_content
            .chapter_by_id_for_analysis(chapter_id)
            .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter::ChapterContentReadApplicationPort for OrchestratorContext {
    async fn analyses(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterAnalysisEntry>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        a4n_db::chapter_content::chapter_analyses_for_project(
            self.chapter_content.database.executor(),
            project_id,
        )
        .await
        .map_err(AppError::from)
    }

    async fn index(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterIndexEntry>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let chapters = self.chapters_for_project(project_id).await?;
        let metadata = chapter_metadata_with_sync_state(self, &chapters).await?;
        Ok(chapters
            .iter()
            .zip(metadata)
            .map(|(chapter, metadata)| {
                a4n_chapter_content_core::chapter_index_entry(chapter, metadata)
            })
            .collect())
    }

    async fn list(&self, user_id: i64, project_id: i64) -> Result<Vec<ChapterSummary>, AppError> {
        self.list_chapter_summaries_for_user(user_id, project_id)
            .await
    }

    async fn summary(
        &self,
        user_id: i64,
        chapter_id: i64,
        project_id: Option<i64>,
    ) -> Result<ChapterSummary, AppError> {
        self.chapter_summary_for_user_in_project(user_id, chapter_id, project_id)
            .await
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

pub(crate) async fn chapter_summaries_with_sync_state(
    state: &OrchestratorContext,
    chapters: &[ChapterRecord],
) -> Result<Vec<ChapterSummary>, AppError> {
    let metadata = chapter_metadata_with_sync_state(state, chapters).await?;
    Ok(chapters
        .iter()
        .zip(metadata)
        .map(|(chapter, metadata)| {
            a4n_chapter_content_core::chapter_summary_with_metadata(chapter, metadata)
        })
        .collect())
}

async fn chapter_metadata_with_sync_state(
    state: &OrchestratorContext,
    chapters: &[ChapterRecord],
) -> Result<Vec<ChapterMetadata>, AppError> {
    let Some(project_id) = chapters.first().map(|chapter| chapter.project_id) else {
        return Ok(Vec::new());
    };
    let chapter_ids = chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let db = &state.chapter_content.database;
    let (analysis_runs_result, content_runs_result) = futures::join!(
        a4n_db::workflow::latest_workflow_run_summaries_for_chapters(
            db,
            project_id,
            "chapter_analysis",
            &chapter_ids,
        ),
        a4n_db::workflow::latest_workflow_run_summaries_for_chapters(
            db,
            project_id,
            a4n_chapter_content_core::CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
            &chapter_ids,
        ),
    );
    let analysis_runs = analysis_runs_result.map_err(crate::workflow::workflow_read_app_error)?;
    let content_runs = content_runs_result.map_err(crate::workflow::workflow_read_app_error)?;
    let run_ids = analysis_runs
        .iter()
        .chain(&content_runs)
        .map(|run| run.id)
        .collect::<Vec<_>>();
    let checkpoints = a4n_db::workflow::workflow_checkpoint_summaries_for_runs(db, &run_ids)
        .await
        .map_err(crate::workflow::workflow_read_app_error)?;
    let mut summaries = chapters
        .iter()
        .map(a4n_chapter_content_core::chapter_metadata)
        .collect::<Vec<_>>();
    a4n_chapter_content_core::attach_chapter_sync_state(
        chapters,
        &mut summaries,
        &analysis_runs,
        &content_runs,
        checkpoints,
    )?;
    Ok(summaries)
}

pub(crate) async fn chapter_summary_with_sync_state(
    state: &OrchestratorContext,
    chapter: &ChapterRecord,
) -> Result<ChapterSummary, AppError> {
    let mut summaries =
        chapter_summaries_with_sync_state(state, std::slice::from_ref(chapter)).await?;
    summaries.pop().ok_or_else(|| AppError::InvalidState {
        current: "empty_chapter_summary_projection".to_string(),
        operation: "读取章节摘要".to_string(),
    })
}
