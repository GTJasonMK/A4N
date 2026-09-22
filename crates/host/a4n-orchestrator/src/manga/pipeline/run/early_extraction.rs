mod characters;
mod events;
mod speech;

use a4n_error::AppError;

use self::characters::run_character_binding_stage;
use self::events::project_shooting_script_events;
use self::speech::{SpeechSelectionStageInput, run_speech_selection_stage};
use super::contracts::{MangaEarlyRun, MangaRunContext};

pub(super) async fn run_early_extraction(
    context: &MangaRunContext<'_>,
) -> Result<MangaEarlyRun, AppError> {
    let events_json = project_shooting_script_events(context)?;
    let characters_json = run_character_binding_stage(context, &events_json).await?;
    let speech_json = run_speech_selection_stage(
        context,
        SpeechSelectionStageInput {
            events_json: &events_json,
            characters_json: &characters_json,
        },
    )
    .await?;

    Ok(MangaEarlyRun {
        events_json,
        characters_json,
        speech_json,
    })
}
