use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use sea_orm::ConnectionTrait;

use crate::generation::generation_error;

use super::super::super::OrchestratorContext;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;
use super::contracts::MangaGenerationTraceRecord;
use super::payload::build_manga_generation_trace_record;

impl OrchestratorContext {
    pub(in crate::manga::output_sync) async fn record_chapter_manga_prompt_generation_run(
        &self,
        row: &chapter_manga_prompt::Model,
        trace: MangaPromptGenerationTrace,
    ) -> Result<(), AppError> {
        if self
            .chapter_manga_prompt_generation_trace_exists(
                self.database.connection(),
                row.id,
                &trace,
            )
            .await?
        {
            return Ok(());
        }
        let MangaGenerationTraceRecord { run, checkpoint } =
            build_manga_generation_trace_record(row, trace)?;
        a4n_db::generation::record_completed_run_with_checkpoint_transaction(
            self.database.connection(),
            a4n_db::generation::RecordCompletedGenerationRunWithCheckpointInput { run, checkpoint },
        )
        .await
        .map_err(generation_error)?;
        Ok(())
    }

    async fn chapter_manga_prompt_generation_trace_exists<C>(
        &self,
        db: &C,
        prompt_id: i64,
        trace: &MangaPromptGenerationTrace,
    ) -> Result<bool, AppError>
    where
        C: ConnectionTrait,
    {
        a4n_db::generation::generation_run_exists_for_target_output(
            db,
            "chapter_manga_prompt",
            prompt_id,
            &trace.prompt_name,
            &trace.response_text,
        )
        .await
        .map_err(generation_error)
    }
}
