mod items;
mod synthesis;
mod visual;

use a4n_core::error::AppError;
use serde_json::json;

use self::items::run_item_extraction_stage;
use self::synthesis::{ComicSynthesisStageInput, run_comic_synthesis_stage};
use self::visual::run_visual_anchor_stage;
use super::super::pipeline_json::required_array_field;
use super::contracts::{MangaEarlyRun, MangaExtractionRun, MangaRunContext};

pub(super) async fn run_late_extraction(
    context: &MangaRunContext<'_>,
    early: MangaEarlyRun,
) -> Result<MangaExtractionRun, AppError> {
    let MangaEarlyRun {
        events_json,
        characters_json,
        speech_json,
    } = early;

    let visual_json = run_visual_anchor_stage(context, &events_json).await?;
    let items_json = run_item_extraction_stage(context, &speech_json, &visual_json).await?;
    let synthesis_json = run_comic_synthesis_stage(
        context,
        ComicSynthesisStageInput {
            events_json: &events_json,
            speech_json: &speech_json,
            visual_json: &visual_json,
            items_json: &items_json,
        },
    )
    .await?;

    let items = required_array_field(&items_json, "items", "manga_item_extraction")?;
    let extraction_summary = json!({
        "characters": characters_json,
        "events": events_json,
        "speech": speech_json,
        "visual_anchors": visual_json,
        "items": items,
        "comic_synthesis": synthesis_json
    });

    Ok(MangaExtractionRun { extraction_summary })
}
