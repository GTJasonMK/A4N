use crate::state::ServerState;
use serde_json::Value;

use super::super::super::{
    ApiError, portrait_generation_actions, strict_runtime_command_input, to_value,
};

macro_rules! portrait_web_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = strict_runtime_command_input(args, stringify!($name))?;
            let preview = $action(state.runtime(), user_id, input)
                .await
                .map_err(ApiError::bad_request)?;
            to_value(preview)
        }
    };
}

portrait_web_preview_command!(
    preview_generate_portrait_request,
    portrait_generation_actions::GeneratePortraitInput,
    portrait_generation_actions::preview_generate_portrait_request,
);

portrait_web_preview_command!(
    preview_regenerate_portrait_request,
    portrait_generation_actions::RegeneratePortraitInput,
    portrait_generation_actions::preview_regenerate_portrait_request,
);

portrait_web_preview_command!(
    preview_generate_age_variant_request,
    portrait_generation_actions::GenerateAgeVariantInput,
    portrait_generation_actions::preview_generate_age_variant_request,
);

portrait_web_preview_command!(
    preview_generate_missing_portraits_request,
    portrait_generation_actions::GenerateMissingPortraitsInput,
    portrait_generation_actions::preview_generate_missing_portraits_request,
);
