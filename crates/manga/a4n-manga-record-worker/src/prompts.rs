use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_domain::manga::{MangaPageSlotsV1, MangaPipelineArtifactStage};
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
        let artifacts = a4n_db::manga::manga_pipeline_artifacts_by_chapter(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(manga_error)?;
        let current_page_numbers = match artifacts.iter().find(|artifact| {
            artifact.stage == MangaPipelineArtifactStage::PageSlots && !artifact.stale
        }) {
            Some(artifact) => {
                let slots = serde_json::from_value::<MangaPageSlotsV1>(artifact.payload.clone())
                    .map_err(|error| {
                        WorkerFailure::invalid_request(format!("当前漫画分页产物无效: {error}"))
                    })?;
                slots.validate().map_err(WorkerFailure::invalid_request)?;
                Some(
                    slots
                        .pages
                        .into_iter()
                        .map(|page| page.page_number)
                        .collect::<std::collections::HashSet<_>>(),
                )
            }
            None => None,
        };
        prompts
            .iter()
            .map(|prompt| {
                let mut summary = a4n_manga_core::manga_page_prompt_summary(prompt)
                    .map_err(WorkerFailure::invalid_request)?;
                summary.stale_stages = a4n_manga_core::manga_page_prompt_stale_stages(
                    prompt.id,
                    prompt.page_number,
                    &artifacts,
                )
                .map_err(WorkerFailure::invalid_request)?;
                summary.orphaned = current_page_numbers
                    .as_ref()
                    .is_some_and(|pages| !pages.contains(&(prompt.page_number as u32)));
                Ok(summary)
            })
            .collect()
    }
}
