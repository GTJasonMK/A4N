mod beat_mapping {
    mod checkpoint;
    mod execution;
    mod preparation;
    mod preview;
    mod request;
}
mod contracts;
mod generation;
mod llm {
    mod checkpoint;
    mod execution;
    mod trace;
}
mod preview;
mod queries;
mod regeneration;
mod requests;

use a4n_db::project as project_db;
use a4n_domain::chapter::{ChapterOutlineRecord, PartOutlineRecord};
use a4n_error::AppError;

use crate::outline::{
    OrchestratorContext,
    chapter_contract::{ensure_focus_part_in_context, validate_requested_part_context},
    context::{ChapterOutlineGenerationContext, load_chapter_outline_generation_context},
};

pub(super) struct PreparedChapterOutlineRegeneration {
    pub(super) old: ChapterOutlineRecord,
    pub(super) context: ChapterOutlineGenerationContext,
    pub(super) focus_part: Option<PartOutlineRecord>,
}

impl OrchestratorContext {
    pub(super) async fn prepare_chapter_outline_regeneration(
        &self,
        project_id: i64,
        outline_id: i64,
    ) -> Result<PreparedChapterOutlineRegeneration, AppError> {
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
        let (context, no_other_outline) = futures::join!(
            load_chapter_outline_generation_context(self.database.connection(), project_id),
            self.ensure_no_other_outline_for_chapter(project_id, chapter_number, outline_id),
        );
        let context = context?;
        validate_requested_part_context(&context.parts, chapter_number, chapter_number)?;
        let focus_part = match old.part_id {
            Some(part_id) => Some(
                context
                    .parts
                    .iter()
                    .find(|part| part.id == part_id)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "章节大纲关联的分部 {part_id} 不属于项目 {project_id}"
                        ))
                    })?,
            ),
            None => None,
        };
        ensure_focus_part_in_context(focus_part.as_ref(), &context.parts)?;
        no_other_outline?;
        Ok(PreparedChapterOutlineRegeneration {
            old,
            context,
            focus_part,
        })
    }
}

#[cfg(test)]
mod tests;
