macro_rules! optimize_web_session_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let session_id = i64_arg(args, "sessionId")?;
            Ok(json!(
                $action(state.runtime(), user_id, session_id)
                    .await
                    .map_err(ApiError::bad_request)?
            ))
        }
    };
}

macro_rules! optimize_web_session_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let session_id = i64_arg(args, "sessionId")?;
            $action(state.runtime(), user_id, session_id)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

macro_rules! optimize_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            Ok(json!(
                $action(state.runtime(), user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?
            ))
        }
    };
}

macro_rules! optimize_web_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let preview = $action(state.runtime(), user_id, input)
                .await
                .map_err(ApiError::bad_request)?;
            to_value(preview)
        }
    };
}

pub(super) use optimize_web_input_command;
pub(super) use optimize_web_preview_command;
pub(super) use optimize_web_session_command;
pub(super) use optimize_web_session_unit_command;
