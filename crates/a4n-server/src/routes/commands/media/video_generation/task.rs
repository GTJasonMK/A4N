macro_rules! video_generation_web_task_command {
    (
        $name:ident,
        $start_message:expr,
        $error_code:expr,
        |$state:ident, $user_id:ident, $args:ident, $input:ident, $scope:ident, $progress:ident, $cancel_token:ident| {
            $($prepare:stmt);* $(;)?
        },
        $run:expr,
        $success_summary:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            $($prepare)*
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskType::VideoGeneration,
                $scope.project_id,
                $scope.total,
                $start_message,
            )
            .await;
            let $cancel_token = task
                .as_ref()
                .map(|handle| handle.cancel_token.clone())
                .unwrap_or_else(tokio_util::sync::CancellationToken::new);
            let mut $progress = web_progress_reporter($state, task.clone(), task_id.clone());
            let result = $run;
            let cancelled = task.as_ref().is_some_and(|handle| handle.is_cancelled());
            let video = finish_web_video_generation_task(
                $state,
                task_id,
                result,
                cancelled,
                $error_code,
                $success_summary,
            )
            .await?;
            Ok(json!(video))
        }
    };
}

pub(super) use video_generation_web_task_command;
