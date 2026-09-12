macro_rules! optimize_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = strict_command_input(args, stringify!($name))?;
            to_value(
                $action(state.runtime(), user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! optimize_web_runtime_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = strict_runtime_command_input(args, stringify!($name))?;
            to_value(
                $action(state.runtime(), user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

pub(super) use optimize_web_input_command;
pub(super) use optimize_web_runtime_input_command;
