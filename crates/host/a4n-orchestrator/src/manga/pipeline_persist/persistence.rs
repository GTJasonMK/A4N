use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_error::AppError;

use super::super::OrchestratorContext;
use super::contracts::MangaPipelinePersistInput;
use super::drafts::pipeline_drafts;

impl OrchestratorContext {
    pub(in crate::manga) async fn persist_pipeline_outputs(
        &self,
        input: MangaPipelinePersistInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let drafts = pipeline_drafts(&input)?;
        self.sync_generated_page_prompts(input.project_id, input.chapter_id, drafts)
            .await
    }
}
