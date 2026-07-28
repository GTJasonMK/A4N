macro_rules! video_prompt_web_task_command {
    ($name:ident, $start_message:expr, $success_message:expr, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input = video_prompt_input_from_args(user_id, args, stringify!($name))?;
            let scope =
                video_prompt_actions::chapter_video_prompt_task_scope(state.runtime(), &input)
                    .await
                    .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskKind::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task
                .as_ref()
                .map(|handle| handle.cancel_token.clone())
                .unwrap_or_else(tokio_util::sync::CancellationToken::new);
            let mut progress = web_progress_reporter(task.clone());
            let mut chunk_reporter = web_chunk_reporter(task.clone());
            let mut chunks = move |content: &str, index: u32| {
                let _ = chunk_reporter(content, index);
            };
            let result = $action(
                state.runtime(),
                input,
                Some(&mut progress),
                Some(&mut chunks),
                cancel_token,
            )
            .await;
            let prompts = finish_web_task(
                state,
                task_id,
                string_action_result(
                    result,
                    task.as_ref().is_some_and(|handle| handle.is_cancelled()),
                ),
            )
            .await?;
            Ok(json!(prompts))
        }
    };
}

pub(super) use video_prompt_web_task_command;
