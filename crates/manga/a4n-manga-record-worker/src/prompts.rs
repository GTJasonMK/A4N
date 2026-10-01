use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_manga_record_worker_contract::{MANGA_RECORD_LIST_OPERATION, MangaChapterInput};
use a4n_worker_contract::WorkerEnvelope;

use super::{
    WorkerState,
    access::{user_id, verify_manga_viewer},
    protocol::{WorkerFailure, manga_error, positive, validate_request},
};

impl WorkerState {
    pub(super) async fn list_page_prompts(
        &self,
        request: WorkerEnvelope<MangaChapterInput>,
    ) -> Result<Vec<MangaPagePromptSummary>, WorkerFailure> {
        validate_request(&request, MANGA_RECORD_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        verify_manga_viewer(&self.database, input.project_id, user_id).await?;
        let prompts = a4n_db::manga::manga_page_prompts_by_chapter(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(manga_error)?;
        prompts
            .iter()
            .map(|prompt| {
                a4n_manga_core::manga_page_prompt_summary(prompt)
                    .map_err(WorkerFailure::invalid_request)
            })
            .collect()
    }
}
