macro_rules! outline_web_generation_command {
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
            let scope = $scope(state.runtime(), user_id, &input).await?;
            let task_id = sanitize_optional_task_id(input.task_id.clone())?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let result = $action(state.runtime(), user_id, input, Some(&progress)).await;
            let value = finish_web_task(state, task_id, result, $success_message).await?;
            to_value(value)
        }
    };
}

macro_rules! outline_web_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state.runtime(), user_id, input).await?)
        }
    };
}

pub(super) use outline_web_generation_command;
pub(super) use outline_web_preview_command;
