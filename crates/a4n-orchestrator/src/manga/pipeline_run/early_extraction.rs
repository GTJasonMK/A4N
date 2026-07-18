mod characters;
mod events;
mod speech;

use a4n_core::error::AppError;
use serde_json::{Map, Value};

use self::characters::run_character_binding_stage;
use self::events::project_shooting_script_events;
use self::speech::{SpeechSelectionStageInput, run_speech_selection_stage};
use super::contracts::{MangaEarlyRun, MangaRunContext};

pub(super) async fn run_early_extraction(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
) -> Result<MangaEarlyRun, AppError> {
    let (events_out, events_json) = project_shooting_script_events(context, checkpoint)?;
    let (_, characters_out, characters_json) =
        run_character_binding_stage(context, checkpoint, &events_json, &events_out).await?;
    let (_, speech_out, speech_json) = run_speech_selection_stage(
        context,
        checkpoint,
        SpeechSelectionStageInput {
            events_json: &events_json,
            events_out: &events_out,
            characters_json: &characters_json,
            characters_out: &characters_out,
        },
    )
    .await?;

    Ok(MangaEarlyRun {
        events_json,
        characters_json,
        speech_out,
        speech_json,
    })
}
