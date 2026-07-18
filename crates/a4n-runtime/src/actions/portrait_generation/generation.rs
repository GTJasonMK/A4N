use super::image_pipeline::PersistGeneratedPortraitInput;
use super::*;

mod initial;
mod missing;
mod scope;
mod variants;

pub use initial::generate_portrait;
pub use missing::generate_missing_portraits;
pub use scope::{
    generate_age_variant_task_scope, generate_missing_portraits_task_scope,
    generate_portrait_task_scope, regenerate_portrait_task_scope,
};
pub use variants::{generate_age_variant, regenerate_portrait};

pub(super) fn age_variant_prompt(prompt: &str, age: &str) -> String {
    format!("{prompt}，age: {age}")
}

pub(super) fn report_progress(
    progress: &mut Option<PortraitProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message).map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub(super) fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), String> {
    if cancel_token.is_cancelled() {
        return Err(AppError::Cancelled.to_string());
    }
    Ok(())
}
