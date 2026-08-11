mod contracts;
mod mapping;
mod records;

pub use contracts::{UpdateVideoConfigInput, VideoConfigInput, VideoConfigSummary};
pub use records::{
    add_video_config, list_video_configs, remove_video_config, set_default_video_config,
    update_video_config,
};
