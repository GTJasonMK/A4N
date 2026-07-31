macro_rules! character_ai_task_command {
    (
        $(#[$meta:meta])*
        $name:ident,
        $scope:path,
        $start_message:expr,
        ($($arg:ident : $arg_ty:ty),* $(,)?),
        |$app_state:ident, $user_id:ident, $project_id:ident, $progress:ident| $run:expr $(,)?
    ) => {
        $(#[$meta])*
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            project_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<serde_json::Value, String> {
            let scope = $scope(state.inner(), user_id, project_id).await?;
            let progress_task = crate::progress_task::WorkflowProgressTask::new(
                state.inner(),
                Some(user_id),
                task_id,
                a4n_core::event::channel::TaskKind::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let cancel_token = progress_task.cancel_token();
            let reporter =
                |current, total, message: &str| progress_task.progress(current, total, message);
            let runtime_progress =
                a4n_runtime::host_api::character_ai::CharacterAiProgress::new(
                    Some(&reporter),
                    Some(&cancel_token),
                    progress_task.task_id(),
                );
            let $app_state = state.inner();
            let $user_id = user_id;
            let $project_id = project_id;
            let $progress = &runtime_progress;
            let result = $run.await;
            progress_task.remove(state.inner());
            result
        }
    };
}

pub(super) use character_ai_task_command;
