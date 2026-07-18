mod items;
mod synthesis;
mod visual;

use a4n_core::error::AppError;

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
        events_out,
        events_json,
        characters_json,
        speech_out,
        speech_json,
    } = early;
    let reference_assets = context.reference_assets.clone();
    let Some((visual_out, visual_json)) =
        preview_visual(context, steps, &events_out, &events_json, &reference_assets).await?
    else {
        return Ok(None);
    };
    let Some((items_out, items_json)) = preview_items(
        context,
        steps,
        ItemPreviewInput {
            speech_out: &speech_out,
            speech_json: &speech_json,
            visual_out: &visual_out,
            visual_json: &visual_json,
            reference_assets: &reference_assets,
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
            events_out: &events_out,
            events_json: &events_json,
            speech_out: &speech_out,
            speech_json: &speech_json,
            visual_out: &visual_out,
            visual_json: &visual_json,
            items_out: &items_out,
            items_json: &items_json,
            reference_assets: &reference_assets,
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
