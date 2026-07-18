macro_rules! web_image_generation_command {
    (
        $name:ident,
        $start_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $args:ident, $progress:ident, $scope:ident| {
            $($prepare:stmt);* $(;)?
        },
        $run:expr $(,)?
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
                TaskType::ImageGeneration,
                $scope.project_id,
                $scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter =
                web_fire_and_forget_progress_reporter($state, task.clone(), task_id.clone());
            let $progress = image_generation_actions::ImageGenerationProgress::new(
                Some(&reporter),
                cancel_token.as_ref(),
            );
            let result = $run;
            let value = finish_web_task(
                $state,
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

pub(super) use web_image_generation_command;
