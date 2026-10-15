use super::{
    WorkerState,
    errors::{WorkerFailure, chapter_error},
    permissions::verify_project_viewer,
    validation::validate_request,
};
use a4n_chapter_content_core::{
    CHAPTER_CONTENT_SYNC_WORKFLOW_KIND, attach_chapter_sync_state, chapter_index_entry,
    chapter_metadata, chapter_summary_with_metadata,
};
use a4n_chapter_content_worker_contract::{
    CHAPTER_CONTENT_ANALYSES_OPERATION, CHAPTER_CONTENT_INDEX_OPERATION,
    CHAPTER_CONTENT_LIST_OPERATION, CHAPTER_CONTENT_STORY_EVENTS_OPERATION,
    CHAPTER_CONTENT_SUMMARY_OPERATION, ChapterAnalysisEntry, ChapterIndexEntry, ChapterInput,
    ChapterMetadata, ChapterStoryEventEntry, ChapterSummary, ProjectInput,
};
use a4n_domain::chapter::ChapterRecord;
use a4n_worker_contract::WorkerEnvelope;

impl WorkerState {
    pub(super) async fn analyses(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ChapterAnalysisEntry>, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_ANALYSES_OPERATION)?;
        let project_id =
            WorkerFailure::require_positive_id(request.payload.project_id, "project_id")?;
        verify_project_viewer(
            &self.database,
            project_id,
            request.principal.require_user_id()?,
        )
        .await?;
        a4n_db::chapter_content::chapter_analyses_for_project(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_database_error)
    }

    pub(super) async fn story_events(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ChapterStoryEventEntry>, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_STORY_EVENTS_OPERATION)?;
        let project_id = positive("project_id", request.payload.project_id)?;
        verify_project_viewer(&self.database, project_id, user_id(&request)?).await?;
        a4n_db::chapter_content::chapter_story_events_for_project(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))
    }

    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ChapterSummary>, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_LIST_OPERATION)?;
        let project_id =
            WorkerFailure::require_positive_id(request.payload.project_id, "project_id")?;
        let chapters = self
            .chapters_for_viewer(request.principal.require_user_id()?, project_id)
            .await?;
        self.project_summaries(project_id, chapters).await
    }

    pub(super) async fn index(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ChapterIndexEntry>, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_INDEX_OPERATION)?;
        let project_id =
            WorkerFailure::require_positive_id(request.payload.project_id, "project_id")?;
        let chapters = self
            .chapters_for_viewer(request.principal.require_user_id()?, project_id)
            .await?;
        let metadata = self.project_metadata(project_id, &chapters).await?;
        Ok(chapters
            .iter()
            .zip(metadata)
            .map(|(chapter, metadata)| chapter_index_entry(chapter, metadata))
            .collect())
    }

    async fn chapters_for_viewer(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterRecord>, WorkerFailure> {
        verify_project_viewer(&self.database, project_id, user_id).await?;
        a4n_db::chapter_content::chapters_for_project(self.database.connection(), project_id)
            .await
            .map_err(a4n_worker_db_adapter::map_database_error)
    }

    pub(super) async fn summary(
        &self,
        request: WorkerEnvelope<ChapterInput>,
    ) -> Result<ChapterSummary, WorkerFailure> {
        validate_request(&request, CHAPTER_CONTENT_SUMMARY_OPERATION)?;
        let chapter_id =
            WorkerFailure::require_positive_id(request.payload.chapter_id, "chapter_id")?;
        let chapter = a4n_db::chapter_content::chapter_by_id_for_analysis(
            self.database.connection(),
            chapter_id,
        )
        .await
        .map_err(chapter_error)?;
        if request
            .project_id
            .is_some_and(|project_id| project_id != chapter.project_id)
        {
            return Err(WorkerFailure::not_found(format!(
                "章节不属于指定项目: {chapter_id}"
            )));
        }
        verify_project_viewer(
            &self.database,
            chapter.project_id,
            request.principal.require_user_id()?,
        )
        .await?;
        let mut summaries = self
            .project_summaries(chapter.project_id, vec![chapter.clone()])
            .await?;
        summaries
            .pop()
            .ok_or_else(|| WorkerFailure::unavailable("章节摘要投影为空"))
    }

    async fn project_summaries(
        &self,
        project_id: i64,
        chapters: Vec<ChapterRecord>,
    ) -> Result<Vec<ChapterSummary>, WorkerFailure> {
        let metadata = self.project_metadata(project_id, &chapters).await?;
        Ok(chapters
            .iter()
            .zip(metadata)
            .map(|(chapter, metadata)| chapter_summary_with_metadata(chapter, metadata))
            .collect())
    }

    async fn project_metadata(
        &self,
        project_id: i64,
        chapters: &[ChapterRecord],
    ) -> Result<Vec<ChapterMetadata>, WorkerFailure> {
        let chapter_ids = chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>();
        let (analysis_runs, content_runs) = tokio::try_join!(
            a4n_db::workflow::latest_workflow_run_summaries_for_chapters(
                &self.database,
                project_id,
                "chapter_analysis",
                &chapter_ids,
            ),
            a4n_db::workflow::latest_workflow_run_summaries_for_chapters(
                &self.database,
                project_id,
                CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
                &chapter_ids,
            ),
        )
        .map_err(a4n_worker_db_adapter::map_workflow_read_error_as_conflict)?;
        let run_ids = analysis_runs
            .iter()
            .chain(&content_runs)
            .map(|run| run.id)
            .collect::<Vec<_>>();
        let checkpoints =
            a4n_db::workflow::workflow_checkpoint_summaries_for_runs(&self.database, &run_ids)
                .await
                .map_err(a4n_worker_db_adapter::map_workflow_read_error_as_conflict)?;
        let mut summaries = chapters.iter().map(chapter_metadata).collect::<Vec<_>>();
        attach_chapter_sync_state(
            chapters,
            &mut summaries,
            &analysis_runs,
            &content_runs,
            checkpoints,
        )
        .map_err(a4n_worker_db_adapter::map_app_error)?;
        Ok(summaries)
    }
}
