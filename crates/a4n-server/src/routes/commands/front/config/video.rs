use super::shared::{
    config_web_id_unit_command, config_web_input_command, config_web_model_discovery_command,
    config_web_no_arg_command, config_web_update_command,
};
use crate::routes::commands::config_records_actions;
use a4n_api_contract::config::{UpdateVideoConfigInput, VideoConfigInput};

config_web_no_arg_command!(
    list_video_configs,
    config_records_actions::list_video_configs,
);
config_web_input_command!(
    add_video_config,
    VideoConfigInput,
    config_records_actions::add_video_config,
);
config_web_update_command!(
    update_video_config,
    UpdateVideoConfigInput,
    config_records_actions::update_video_config,
);
config_web_id_unit_command!(
    remove_video_config,
    config_records_actions::remove_video_config,
);
config_web_id_unit_command!(
    set_default_video_config,
    config_records_actions::set_default_video_config,
);

config_web_model_discovery_command!(
    detect_video_config_models,
    config_records_actions::detect_video_config_models,
);
