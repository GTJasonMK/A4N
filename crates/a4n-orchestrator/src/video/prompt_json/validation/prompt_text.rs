mod density;
mod references;
mod sections;

pub(in crate::video::prompt_json) use density::validate_inserted_segment_draft;
pub(in crate::video::prompt_json::validation) use density::{
    required_min_prompt_text, required_text, validate_optional_prompt_text,
};
pub(in crate::video) use references::direct_prompt_internal_reference_term;
pub use references::{validate_direct_prompt_text, validate_reference_label_text};
pub(in crate::video::prompt_json::validation) use sections::{
    validate_motion_prompt_has_no_temporal_storyboard, validate_prompt_has_required_sections,
};
pub(in crate::video) use sections::{
    validate_prompt_has_no_upload_status, validate_prompt_has_shot_storyboard_section,
};
pub use sections::{
    video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
};
