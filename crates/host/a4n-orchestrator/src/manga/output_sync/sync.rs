use a4n_db::manga::{sync_manga_page_prompts_by_chapter, upsert_manga_page_prompt};
use a4n_domain::manga::{MangaPagePromptRecord, MangaPagePromptSyncDraft};
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
    ) -> Result<Vec<MangaPagePromptRecord>, AppError> {
        let now = Utc::now().naive_utc();
        let traces = drafts
            .iter()
            .map(|draft| draft.generation_traces.clone())
            .collect::<Vec<_>>();
        let sync_drafts = drafts
            .into_iter()
            .map(|draft| MangaPagePromptSyncDraft {
                chapter_id: draft.chapter_id,
                project_id: draft.project_id,
                page_number: draft.page_number,
                image_prompt: draft.image_prompt,
                dialogue_text: draft.dialogue_text,
                characters_in_frame: draft.characters_in_frame,
                layout: draft.layout,
                status: draft.status,
            })
            .collect();
        let rows = sync_manga_page_prompts_by_chapter(
            self.database.connection(),
            project_id,
            chapter_id,
            sync_drafts,
            now,
        )
        .await
        .map_err(manga_pipeline_db_error)?;
        if rows.len() != traces.len() {
            return Err(AppError::InvalidState {
                current: format!("rows={}, traces={}", rows.len(), traces.len()),
                operation: "同步漫画页面并关联生成记录".to_string(),
            });
        }
        for (row, traces) in rows.iter().zip(traces) {
            for trace in traces {
                self.record_manga_page_prompt_generation_run(row, trace)
                    .await?;
            }
        }
        Ok(rows)
    }

    pub(in crate::manga) async fn upsert_generated_page_prompt(
        &self,
        project_id: i64,
        chapter_id: i64,
        draft: MangaPagePromptDraft,
    ) -> Result<MangaPagePromptRecord, AppError> {
        let now = Utc::now().naive_utc();
        let traces = draft.generation_traces.clone();
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
        let row = upsert_manga_page_prompt(
            self.database.connection(),
            project_id,
            chapter_id,
            sync,
            now,
        )
        .await
        .map_err(manga_pipeline_db_error)?;
        for trace in traces {
            self.record_manga_page_prompt_generation_run(&row, trace)
                .await?;
        }
        Ok(row)
    }
}
