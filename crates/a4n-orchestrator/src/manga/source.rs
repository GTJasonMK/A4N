use a4n_core::error::AppError;

use crate::{
    OrchestratorContext,
    project::{ChapterMangaSourceSummary, MangaCharacterPortraitRefSummary},
};

use super::{MangaCharacterPortraitRef, MangaReferenceContext};

pub fn ensure_manga_source_can_write(
    source: &ChapterMangaSourceSummary,
    operation: &str,
) -> Result<(), AppError> {
    if source.status == "deleted" {
        return Err(AppError::InvalidArgument(format!(
            "第 {} 章已删除，无法{operation}",
            source.chapter_number
        )));
    }
    if source.status == "generating" {
        return Err(AppError::InvalidArgument(format!(
            "第 {} 章正在生成中，请等待完成后再{operation}",
            source.chapter_number
        )));
    }
    Ok(())
}

impl OrchestratorContext {
    pub(crate) async fn manga_reference_context(
        &self,
        project_id: i64,
    ) -> Result<MangaReferenceContext, AppError> {
        self.manga_character_portrait_refs(project_id)
            .await
            .map(manga_reference_context)
    }
}

fn manga_reference_context(
    summaries: Vec<MangaCharacterPortraitRefSummary>,
) -> MangaReferenceContext {
    MangaReferenceContext {
        character_portraits: summaries
            .into_iter()
            .map(|summary| MangaCharacterPortraitRef {
                profile_id: summary.profile_id,
                portrait_id: summary.portrait_id,
                character_name: summary.character_name,
                image_path: summary.image_path,
                prompt: summary.prompt,
                portrait_type: summary.portrait_type,
                expression: summary.expression,
                pose: summary.pose,
                is_primary: summary.is_primary,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests;
