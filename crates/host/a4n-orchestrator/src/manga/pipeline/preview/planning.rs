mod page_pipeline;
mod page_slots;

use a4n_error::AppError;
use serde_json::json;

use super::super::json::required_array_field;
use super::contracts::{
    MangaExtractionPreview, MangaPipelineRequestPreviewStep, MangaPreviewContext,
};
use page_pipeline::preview_page_pipeline;
use page_slots::preview_page_slots;

pub(super) async fn preview_planning(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    extraction: MangaExtractionPreview,
) -> Result<(), AppError> {
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
    let Some(slots) = preview_page_slots(context, steps, &extraction_summary).await? else {
        return Ok(());
    };
    if matches!(
        context.stop_after,
        Some(
            a4n_api_contract::manga::MangaPipelineStopAfter::PageSlots
                | a4n_api_contract::manga::MangaPipelineStopAfter::PageContent
        )
    ) {
        return Ok(());
    }
    preview_page_pipeline(context, steps, &slots, &extraction_summary).await
}
