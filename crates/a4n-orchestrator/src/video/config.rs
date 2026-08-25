mod contracts;
mod validation;

pub use self::contracts::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
    VideoConfigParams,
};
pub use self::validation::{
    duration_capability_for_config, validate_video_config_model, validate_video_config_params,
    validate_video_duration_for_config_secs, validate_video_max_concurrent,
};

#[cfg(test)]
mod tests;
