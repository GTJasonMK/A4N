use a4n_db::manga::sync_manga_page_prompts_by_chapter;
use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_domain::manga::MangaPagePromptSyncDraft;
use a4n_error::AppError;
use chrono::Utc;

use super::super::OrchestratorContext;
use super::super::record_errors::manga_pipeline_db_error;
use super::contracts::MangaPagePromptDraft;

impl OrchestratorContext {
    pub(in crate::manga) async fn sync_generated_page_prompts(
        &self,
        project_id: i64,
        chapter_id: i64,
        drafts: Vec<MangaPagePromptDraft>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let now = Utc::now().naive_utc();
        let mut trace_bindings = Vec::new();
        let sync_drafts = drafts
            .into_iter()
            .map(|draft| {
                let generation_traces = draft.generation_traces;
                let sync = MangaPagePromptSyncDraft {
                    chapter_id: draft.chapter_id,
                    project_id: draft.project_id,
                    page_number: draft.page_number,
                    image_prompt: draft.image_prompt,
                    dialogue_text: draft.dialogue_text,
                    characters_in_frame: draft.characters_in_frame,
                    layout: draft.layout,
                    status: draft.status,
                };
                (sync, generation_traces)
            })
            .collect::<Vec<_>>();

        let rows = sync_manga_page_prompts_by_chapter(
            &self.database,
            project_id,
            chapter_id,
            sync_drafts.iter().map(|(draft, _)| draft.clone()).collect(),
            now,
        )
        .await
        .map_err(manga_pipeline_db_error)?;

        for (row, (_, traces)) in rows.iter().zip(sync_drafts) {
            for trace in traces {
                trace_bindings.push((row.clone(), trace));
            }
        }
        for (row, trace) in trace_bindings {
            self.record_manga_page_prompt_generation_run(&row, trace)
                .await?;
        }
        Ok(rows)
    }
}
