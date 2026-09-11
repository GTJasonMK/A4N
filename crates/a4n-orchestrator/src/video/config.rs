mod contracts;
mod validation;

pub use self::contracts::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
};
pub use self::validation::{
    duration_capability_for_config, duration_capability_for_provider_config,
    validate_video_config_model, validate_video_duration_for_config_secs,
    validate_video_duration_for_provider_config_secs, validate_video_provider_config,
    video_provider_config_from_model,
};

#[cfg(test)]
mod tests;
