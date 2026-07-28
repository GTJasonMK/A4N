use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_db::manga::{MangaPromptSyncDraft, sync_manga_prompts_by_chapter};
use chrono::Utc;

use super::super::OrchestratorContext;
use super::contracts::MangaPromptDraft;
use super::errors::manga_pipeline_db_error;

impl OrchestratorContext {
    pub(in crate::manga) async fn sync_generated_outputs_by_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
        drafts: Vec<MangaPromptDraft>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let now = Utc::now().naive_utc();
        let mut trace_bindings = Vec::new();
        let sync_drafts = drafts
            .into_iter()
            .map(|draft| {
                let generation_traces = draft.generation_traces;
                let sync = MangaPromptSyncDraft {
                    chapter_id: draft.chapter_id,
                    project_id: draft.project_id,
                    page_number: draft.page_number,
                    panel_number: draft.panel_number,
                    prompt_type: draft.prompt_type,
                    image_prompt: draft.image_prompt,
                    dialogue_text: draft.dialogue_text,
                    characters_in_frame: draft.characters_in_frame,
                    layout: draft.layout,
                    camera_angle: draft.camera_angle,
                    dialogue_position: draft.dialogue_position,
                    status: draft.status,
                };
                (sync, generation_traces)
            })
            .collect::<Vec<_>>();

        let rows = sync_manga_prompts_by_chapter(
            self.database.connection(),
            project_id,
            chapter_id,
            sync_drafts.iter().map(|(draft, _)| draft.clone()).collect(),
            now,
        )
        .await
        .map_err(manga_pipeline_db_error)?;

        for (row, (_, traces)) in rows.iter().zip(sync_drafts.into_iter()) {
            for trace in traces {
                trace_bindings.push((row.clone(), trace));
            }
        }
        for (row, trace) in trace_bindings {
            self.record_chapter_manga_prompt_generation_run(
                self.database.connection(),
                &row,
                trace,
            )
            .await?;
        }
        self.sync_project_asset_lineage(project_id).await?;
        Ok(rows)
    }
}
