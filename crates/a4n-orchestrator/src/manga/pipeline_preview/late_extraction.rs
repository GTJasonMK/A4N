mod items;
mod synthesis;
mod visual;

use a4n_error::AppError;

use super::contracts::{
    MangaEarlyExtraction, MangaExtractionPreview, MangaPipelineRequestPreviewStep,
    MangaPreviewContext,
};
use items::{ItemPreviewInput, preview_items};
use synthesis::{SynthesisPreviewInput, preview_synthesis};
use visual::preview_visual;

pub(super) async fn preview_late_extraction(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    early: MangaEarlyExtraction,
) -> Result<Option<MangaExtractionPreview>, AppError> {
    let MangaEarlyExtraction {
        events_json,
        characters_json,
        speech_json,
    } = early;
    let Some(visual_json) = preview_visual(context, steps, &events_json).await? else {
        return Ok(None);
    };
    let Some(items_json) = preview_items(
        context,
        steps,
        ItemPreviewInput {
            speech_json: &speech_json,
            visual_json: &visual_json,
        },
    )
    .await?
    else {
        return Ok(None);
    };
    let Some(comic_synthesis_json) = preview_synthesis(
        context,
        steps,
        SynthesisPreviewInput {
            events_json: &events_json,
            speech_json: &speech_json,
            visual_json: &visual_json,
            items_json: &items_json,
        },
    )
    .await?
    else {
        return Ok(None);
    };

    Ok(Some(MangaExtractionPreview {
        characters_json,
        events_json,
        speech_json,
        visual_json,
        items_json,
        comic_synthesis_json,
    }))
}
