use super::append_provider_input_note;
use crate::actions::video_generation::VideoProviderRequestPurpose;

mod binding;
mod cliproxy;
mod transport;
mod workflow;

pub(in crate::actions::video_generation) use self::binding::*;
pub(in crate::actions::video_generation) use self::cliproxy::*;
pub(in crate::actions::video_generation) use self::transport::*;
pub(in crate::actions::video_generation) use self::workflow::*;

fn reject_generation_or_note_preview(
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
    message: String,
    preview_suffix: &str,
) -> Result<(), String> {
    if purpose == VideoProviderRequestPurpose::Generate {
        return Err(message);
    }
    append_provider_input_note(note, format!("{message}{preview_suffix}"));
    Ok(())
}
