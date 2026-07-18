mod all_profiles;
mod analysis;
mod execution;

pub(crate) use all_profiles::sync_all_character_profiles_preview_steps;
pub use all_profiles::sync_all_character_profiles_to_chapter;
pub(crate) use analysis::sync_analysis_character_profiles_to_chapter;

use super::CharacterAiProgress;

fn report_character_workflow_progress(
    progress: Option<&CharacterAiProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.report(current, total, message, cancelled_message)?;
    }
    Ok(())
}
