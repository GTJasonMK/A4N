mod drawing;
mod page_plan;

use a4n_core::error::AppError;
use serde_json::json;

use super::super::pipeline_json::required_array_field;
use super::contracts::{
    MangaExtractionPreview, MangaPipelineRequestPreviewStep, MangaPreviewContext,
};
use drawing::preview_page_drawing_prompts;
use page_plan::preview_page_plan;

pub(super) async fn preview_planning(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction: MangaExtractionPreview,
) -> Result<(), AppError> {
    let reference_assets = context.reference_assets.clone();
    let MangaExtractionPreview {
        characters_json,
        events_json,
        speech_json,
        visual_json,
        items_json,
        comic_synthesis_json,
    } = extraction;
    let items = required_array_field(&items_json, "items", "manga_item_extraction")?;
    let extraction_summary = json!({
        "characters": characters_json,
        "events": events_json,
        "speech": speech_json,
        "visual_anchors": visual_json,
        "items": items,
        "comic_synthesis": comic_synthesis_json
    });
    let Some(plan) = preview_page_plan(context, steps, &extraction_summary).await? else {
        return Ok(());
    };
    preview_page_drawing_prompts(
        context,
        steps,
        &plan,
        &extraction_summary,
        &reference_assets,
    )
    .await
}
