mod characters;
mod events;
mod speech;

use a4n_error::AppError;

use super::contracts::{
    MangaEarlyExtraction, MangaPipelineRequestPreviewStep, MangaPreviewContext,
};
use characters::preview_characters;
use events::preview_events;
use speech::{SpeechPreviewInput, preview_speech};

pub(super) async fn preview_early_extraction(
    context: &MangaPreviewContext<'_>,
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
) -> Result<Option<MangaEarlyExtraction>, AppError> {
    let reference_assets = context.reference_assets.clone();
    let events_json = preview_events(context)?;
    let Some(characters_json) =
        preview_characters(context, steps, &events_json, &reference_assets).await?
    else {
        return Ok(None);
    };
    let Some(speech_json) = preview_speech(
        context,
        steps,
        SpeechPreviewInput {
            events_json: &events_json,
            characters_json: &characters_json,
            reference_assets: &reference_assets,
        },
    )
    .await?
    else {
        return Ok(None);
    };

    Ok(Some(MangaEarlyExtraction {
        events_json,
        characters_json,
        speech_json,
    }))
}
