macro_rules! portrait_web_generation_command {
    (
        $name:ident,
        |$args:ident| $input:expr,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let input = $input;
            let scope = $scope(state.runtime(), user_id, &input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::ImageGeneration,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task
                .as_ref()
                .map(|handle| handle.cancel_token.clone())
                .unwrap_or_else(tokio_util::sync::CancellationToken::new);
            let mut progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let result = $action(
                state.runtime(),
                user_id,
                input,
                Some(&mut progress),
                cancel_token,
            )
            .await;
            let value = finish_web_task(
                state,
                task_id,
                string_action_result(
                    result,
                    task.as_ref().is_some_and(|handle| handle.is_cancelled()),
                ),
                $success_message,
            )
            .await?;
            Ok(json!(value))
        }
    };
}

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

pub(super) use portrait_web_generation_command;
pub(super) use portrait_web_preview_command;
