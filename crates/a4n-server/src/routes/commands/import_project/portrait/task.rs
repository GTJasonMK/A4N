macro_rules! portrait_web_preview_command {
    ($name:ident, |$args:ident| $input:expr, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let preview = $action(state.runtime(), user_id, $input)
                .await
                .map_err(ApiError::bad_request)?;
            to_value(preview)
        }
    };
}

pub(super) use portrait_web_preview_command;
