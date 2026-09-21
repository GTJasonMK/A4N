mod validation;

pub use self::validation::{
    duration_capability_for_provider_config, validate_video_config_model,
    validate_video_duration_for_provider_config_secs, video_provider_config_from_model,
};
pub use a4n_video::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
};

#[cfg(test)]
mod tests;
