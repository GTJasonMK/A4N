use serde_json::{Value, json};

use super::shared::{
    config_web_id_unit_command, config_web_id_value_command, config_web_input_command,
    config_web_no_arg_command, config_web_update_command,
};
use crate::routes::commands::{ApiError, ServerState, config_records_actions};

config_web_no_arg_command!(
    list_embedding_config,
    config_records_actions::list_embedding_config,
);
config_web_input_command!(
    create_embedding_config,
    config_records_actions::CreateEmbeddingInput,
    config_records_actions::create_embedding_config,
);
config_web_update_command!(
    update_embedding_config,
    config_records_actions::UpdateEmbeddingInput,
    config_records_actions::update_embedding_config,
);
config_web_id_unit_command!(
    set_default_embedding_config,
    config_records_actions::set_default_embedding_config,
);
config_web_id_unit_command!(
    delete_embedding_config,
    config_records_actions::delete_embedding_config,
);
config_web_id_value_command!(
    test_embedding_config,
    config_records_actions::test_embedding_config,
);
config_web_id_value_command!(
    preview_test_embedding_config_request,
    config_records_actions::preview_test_embedding_config_request,
);

pub(in crate::routes::commands) async fn list_embedding_backends(
    _state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(config_records_actions::list_embedding_backends()))
}
