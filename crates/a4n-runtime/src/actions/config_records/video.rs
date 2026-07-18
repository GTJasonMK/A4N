mod connectivity;
mod contracts;
mod discovery;
mod mapping;
mod records;

pub use connectivity::test_video_config;
pub use contracts::{UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary};
pub use discovery::detect_video_config_models;
pub use records::{
    activate_video_config, add_video_config, list_video_configs, remove_video_config,
    update_video_config,
};
