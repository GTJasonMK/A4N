use crate::AppState;

pub async fn start_chapter_shooting_script_generation(
    state: &AppState,
    user_id: i64,
    input: a4n_api_contract::shooting_script::StartChapterShootingScriptGenerationInput,
) -> Result<
    a4n_api_contract::generation_tasks::GenerationTaskHandle,
    a4n_api_contract::generation_tasks::GenerationTaskStartError,
> {
    let start = state
        .shooting_script_generation_application()
        .start(user_id, &input)
        .await?;
    if start.should_run {
        let worker = state.clone();
        state.spawn_generation_execution(
            start.handle.clone(),
            "chapter_shooting_script",
            move |handle| async move {
                worker
                    .shooting_script_generation_application()
                    .run(user_id, input, handle)
                    .await
            },
        );
    }
    Ok(start.handle)
}
