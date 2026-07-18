use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;

use super::super::{MANGA_PAGE_PROMPT_TYPE, OrchestratorContext};
use super::contracts::MangaPipelinePersistInput;
use super::drafts::pipeline_drafts;

impl OrchestratorContext {
    pub(in crate::manga) async fn persist_pipeline_outputs(
        &self,
        input: MangaPipelinePersistInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let drafts = pipeline_drafts(&input)?;
        for _ in &input.page_prompts {
            self.ensure_pipeline_not_cancelled(input.project_id, input.chapter_id)
                .await?;
        }

        Ok(self
            .sync_generated_outputs_by_chapter(input.project_id, input.chapter_id, drafts)
            .await?
            .into_iter()
            .filter(|row| row.prompt_type == MANGA_PAGE_PROMPT_TYPE)
            .collect())
    }
}
