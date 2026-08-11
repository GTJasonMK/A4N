use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_db::manga::{MangaPagePromptSyncDraft, sync_manga_page_prompts_by_chapter};
use chrono::Utc;

use super::super::OrchestratorContext;
use super::contracts::MangaPagePromptDraft;
use super::errors::manga_pipeline_db_error;

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
            self.database.connection(),
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
        self.sync_project_asset_lineage_after_commit(project_id, "同步漫画输出资产谱系")
            .await;
        Ok(rows)
    }
}
