use a4n_core::chapter_analysis::ensure_chapter_has_analysis_content;
use a4n_core::error::AppError;
use a4n_db::chapter_content::{
    StoreChapterAnalysisRecordInput, chapter_by_id_for_analysis, store_chapter_analysis_record,
};
use chrono::Utc;
use serde_json::Value;

use super::super::owner::chapter_analysis_storage_projection;
use super::super::{
    ChapterAnalysisInput, ChapterAnalysisProgress, OrchestratorContext, chapter_content_edit_error,
};
use crate::generation::generation_error;

impl OrchestratorContext {
    pub(crate) async fn analyze_and_store_chapter(
        &self,
        input: ChapterAnalysisInput<'_>,
    ) -> Result<Value, AppError> {
        let ChapterAnalysisInput {
            chapter_id,
            prompt_renderer,
            progress,
            stream_reporter,
            cancel_token,
            workflow_run_id,
        } = input;
        let chapter = chapter_by_id_for_analysis(self.database.connection(), chapter_id)
            .await
            .map_err(chapter_content_edit_error)?;
        ensure_chapter_has_analysis_content(&chapter)?;

        let mut progress = ChapterAnalysisProgress::new(progress, cancel_token)
            .with_stream_reporter(stream_reporter);
        let run = self
            .analyze_chapter(&chapter, prompt_renderer, &mut progress, workflow_run_id)
            .await?;
        let analysis = run.analysis.clone();
        let projection = chapter_analysis_storage_projection(&chapter, &analysis);
        store_chapter_analysis_record(
            self.database.connection(),
            chapter.clone(),
            StoreChapterAnalysisRecordInput {
                analysis_data: analysis.clone(),
                summary: projection.summary,
                style_tags: projection.style_tags,
                now: Utc::now().naive_utc(),
            },
        )
        .await
        .map_err(chapter_content_edit_error)?;
        a4n_db::generation::delete_generation_run_targets_for(
            self.database.connection(),
            "chapter_analysis",
            &[chapter.id],
        )
        .await
        .map_err(generation_error)?;
        self.record_chapter_analysis_generation_run(&chapter, &run)
            .await?;
        Ok(analysis)
    }
}
