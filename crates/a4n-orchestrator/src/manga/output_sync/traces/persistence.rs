use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_error::AppError;

use super::super::super::OrchestratorContext;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;
use super::contracts::MangaPageGenerationTraceRecord;
use super::payload::build_manga_page_generation_trace_record;

impl OrchestratorContext {
    pub(in crate::manga::output_sync) async fn record_manga_page_prompt_generation_run(
        &self,
        row: &chapter_manga_prompt::Model,
        trace: MangaPromptGenerationTrace,
    ) -> Result<(), AppError> {
        if self
            .generation_run_exists_for_target_output(
                "chapter_manga_prompt",
                row.id,
                &trace.prompt_name,
                &trace.response_text,
            )
            .await?
        {
            return Ok(());
        }
        let MangaPageGenerationTraceRecord { run, checkpoint } =
            build_manga_page_generation_trace_record(row, trace)?;
        self.record_completed_generation_run_with_checkpoint(
            a4n_db::generation::RecordCompletedGenerationRunWithCheckpointInput { run, checkpoint },
        )
        .await?;
        Ok(())
    }
}
