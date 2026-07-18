use a4n_core::{
    error::AppError,
    source_spans::{ChapterSourceContext, source_metadata},
};

use super::{MangaPipelineStartStage, MangaReferenceContext};

pub fn parse_manga_pipeline_start_stage(
    value: Option<String>,
) -> Result<Option<MangaPipelineStartStage>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    MangaPipelineStartStage::parse(value)
        .map(Some)
        .ok_or_else(|| AppError::InvalidArgument(format!("未知漫画流水线起始步骤: {value}")))
}

pub fn manga_pipeline_business_key(project_id: i64, chapter_id: i64) -> String {
    format!("manga_pipeline:{project_id}:{chapter_id}")
}

pub fn manga_pipeline_fingerprint(
    content: &str,
    reference_context: &MangaReferenceContext,
) -> serde_json::Value {
    serde_json::json!({
        "source": source_metadata(ChapterSourceContext { content }),
        "reference_assets": {
            "character_portrait_count": reference_context.character_portraits.len(),
            "character_portrait_ids": reference_context
                .character_portraits
                .iter()
                .map(|portrait| portrait.portrait_id)
                .collect::<Vec<_>>(),
        }
    })
}

#[cfg(test)]
mod tests;
