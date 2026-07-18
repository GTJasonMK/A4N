mod items;
mod synthesis;
mod visual;

use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use self::items::run_item_extraction_stage;
use self::synthesis::{ComicSynthesisStageInput, run_comic_synthesis_stage};
use self::visual::run_visual_anchor_stage;
use super::contracts::{MangaEarlyRun, MangaExtractionRun, MangaRunContext};

pub(super) async fn run_late_extraction(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
    early: MangaEarlyRun,
) -> Result<MangaExtractionRun, AppError> {
    let MangaEarlyRun {
        events_json,
        characters_json,
        speech_out,
        speech_json,
    } = early;
    let events_out = serde_json::to_string(&json!({ "events": &events_json }))?;

    let (_, visual_out, visual_json) =
        run_visual_anchor_stage(context, checkpoint, &events_json, &events_out).await?;
    let (_, items_out, items_json) = run_item_extraction_stage(
        context,
        checkpoint,
        &speech_json,
        &speech_out,
        &visual_json,
        &visual_out,
    )
    .await?;
    let (_, synthesis_json) = run_comic_synthesis_stage(
        context,
        checkpoint,
        ComicSynthesisStageInput {
            events_json: &events_json,
            events_out: &events_out,
            speech_json: &speech_json,
            speech_out: &speech_out,
            visual_json: &visual_json,
            visual_out: &visual_out,
            items_json: &items_json,
            items_out: &items_out,
        },
    )
    .await?;

    let extraction_summary = json!({
        "characters": characters_json,
        "events": events_json,
        "reference_assets": context.reference_assets.clone(),
        "speech": speech_json,
        "visual_anchors": visual_json,
        "items": items_json.get("items").cloned().unwrap_or_else(|| json!([])),
        "comic_synthesis": synthesis_json
    });

    Ok(MangaExtractionRun { extraction_summary })
}
