macro_rules! character_web_input_command {
    ($name:ident, $input:ty, $action:path, $input_parser:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &$crate::state::ServerState,
            user_id: i64,
            args: &serde_json::Value,
        ) -> Result<serde_json::Value, $crate::routes::error::ApiError> {
            let input: $input = $input_parser(args, stringify!($name))?;
            $crate::routes::commands::to_value(
                $action(state.runtime(), user_id, input)
                    .await
                    .map_err($crate::routes::error::ApiError::bad_request)?,
            )
        }
    };
}

pub(super) use character_web_input_command;
