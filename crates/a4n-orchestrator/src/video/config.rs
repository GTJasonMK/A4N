mod contracts;
mod provider;
mod validation;

pub use self::contracts::{UpdateVideoConfigParams, VideoConfigParams};
pub use self::provider::pick_provider_for_config;
pub use self::validation::{
    duration_capability_for_config, validate_video_config_model, validate_video_config_params,
    validate_video_duration_for_config_secs, validate_video_max_concurrent,
};

#[cfg(test)]
mod tests;
