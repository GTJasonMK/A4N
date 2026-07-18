macro_rules! optimize_web_session_stream_task_command {
    ($name:ident, $scope:path, $start_message:expr, $success_message:expr, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let session_id = i64_arg(args, "sessionId")?;
            let scope = $scope(state.runtime(), user_id, session_id)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let mut report_progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let mut report_stream = web_chunk_reporter(state, task.clone(), task_id.clone());
            let result = $action(
                state.runtime(),
                user_id,
                session_id,
                Some(&mut report_progress),
                Some(&mut report_stream),
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

macro_rules! optimize_web_input_stream_task_command {
    (
        $name:ident,
        $input:ty,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let scope = $scope(state.runtime(), user_id, &input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let mut report_progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let mut report_stream = web_chunk_reporter(state, task.clone(), task_id.clone());
            let result = $action(
                state.runtime(),
                user_id,
                input,
                Some(&mut report_progress),
                Some(&mut report_stream),
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

macro_rules! optimize_web_input_progress_task_command {
    (
        $name:ident,
        $input:ty,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let scope = $scope(state.runtime(), user_id, &input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let mut report_progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let result = $action(state.runtime(), user_id, input, Some(&mut report_progress)).await;
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

pub(super) use optimize_web_input_progress_task_command;
pub(super) use optimize_web_input_stream_task_command;
pub(super) use optimize_web_preview_command;
pub(super) use optimize_web_session_stream_task_command;
