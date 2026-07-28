use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use a4n_db::project as project_db;
use chrono::Utc;

use crate::outline::chapter_contract::{
    ensure_focus_part_in_context, requested_part_for_chapter, update_chapter_outline_from_json,
    validate_requested_part_context,
};
use crate::outline::context::load_chapter_outline_generation_context;
use crate::outline::{ChapterOutlineRegenerationInput, OrchestratorContext, report_progress};

use super::contracts::SingleChapterOutlineJsonInput;

impl OrchestratorContext {
    pub async fn regenerate_chapter_outline(
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
        let old = project_db::chapter_outline_by_id(self.database.connection(), outline_id)
            .await
            .map_err(crate::project::project_outline_record_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {outline_id}")))?;
        if old.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "章节大纲 {outline_id} 不属于项目 {project_id}"
            )));
        }

        let chapter_number = old.chapter_number;
        let context =
            load_chapter_outline_generation_context(self.database.connection(), project_id).await?;
        validate_requested_part_context(&context.parts, chapter_number, chapter_number)?;
        let focus_part = match old.part_id {
            Some(part_id) => {
                let part = self.get_part(part_id).await?;
                if part.project_id != project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "章节大纲关联的分部 {part_id} 不属于项目 {project_id}"
                    )));
                }
                Some(part)
            }
            None => None,
        };
        ensure_focus_part_in_context(focus_part.as_ref(), &context.parts)?;
        self.ensure_no_other_outline_for_chapter(project_id, chapter_number, outline_id)
            .await?;

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
        let updated = update_chapter_outline_from_json(
            self.database.connection(),
            old,
            part_id,
            &generated.chapter_json,
            Utc::now().naive_utc(),
        )
        .await?;
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
