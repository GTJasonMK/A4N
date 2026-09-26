use a4n_db_models::models::chapter::chapter_outline;
use a4n_error::AppError;
use chrono::Utc;

use crate::outline::chapter_contract::{
    requested_part_for_chapter, update_chapter_outline_from_json_in,
};
use crate::outline::context::lock_outline_generation_sources;
use crate::outline::{ChapterOutlineRegenerationInput, OrchestratorContext, report_progress};

use super::contracts::SingleChapterOutlineJsonInput;

impl OrchestratorContext {
    pub(in crate::outline) async fn regenerate_chapter_outline(
        &self,
        input: ChapterOutlineRegenerationInput<'_, '_>,
    ) -> Result<chapter_outline::Model, AppError> {
        let ChapterOutlineRegenerationInput {
            project_id,
            outline_id,
            prompt_renderer,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let prepared = self
            .prepare_chapter_outline_regeneration(project_id, outline_id)
            .await?;
        let old = prepared.old;
        let chapter_number = old.chapter_number;
        let context = prepared.context;
        let focus_part = prepared.focus_part;

        report_progress(
            progress,
            0,
            1,
            &format!("正在重新生成第 {chapter_number} 章章纲"),
        )?;
        let generated = self
            .generate_single_chapter_outline_json(SingleChapterOutlineJsonInput {
                project_id,
                context: &context,
                focus_part: focus_part.as_ref(),
                chapter_number,
                prompt_renderer,
                variables,
                workflow_run_id,
            })
            .await?;
        let part_id =
            requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);
        let tx =
            lock_outline_generation_sources(&self.database, project_id, &context.source_snapshot)
                .await?;
        let updated = update_chapter_outline_from_json_in(
            &tx,
            old,
            part_id,
            &generated.chapter_json,
            Utc::now().naive_utc(),
        )
        .await?;
        a4n_db::commit_transaction(tx).await?;
        self.record_chapter_outline_generation_run(
            &updated,
            &generated.request,
            &generated.response_content,
            generated.finish_reason.as_deref(),
            "重新生成章节大纲",
        )
        .await?;
        report_progress(
            progress,
            1,
            1,
            &format!("已重新生成第 {chapter_number} 章章纲"),
        )?;
        Ok(updated)
    }
}
