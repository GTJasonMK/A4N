use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::host_api::chapter_generation::{
    GenerateChapterStreamInput, chapter_stream_task_id,
    generate_chapter_stream as run_generate_chapter_stream, generate_chapter_stream_task_scope,
};
use tauri::State;

#[tauri::command]
pub async fn generate_chapter_stream(
    state: State<'_, AppState>,
    window: tauri::Window,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    task_id: Option<String>,
    writing_notes: Option<String>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
    client_rag_context: Option<String>,
) -> Result<String, String> {
    let input = GenerateChapterStreamInput {
        project_id,
        chapter_number,
        task_id,
        writing_notes,
        min_word_count,
        max_word_count,
        client_rag_context,
    };
    let scope = generate_chapter_stream_task_scope(state.inner(), user_id, &input).await?;
    let task_id = chapter_stream_task_id(project_id, chapter_number, input.task_id.clone())?;
    let progress_task = WorkflowProgressTask::new(
        window,
        state.inner(),
        Some(task_id.clone()),
        TaskType::LlmCall,
        Some(scope.project_id),
        scope.total,
        "正在启动章节流式生成",
    );
    let cancel_token = progress_task.cancel_token();
    let result = {
        let mut progress = |current: u32, total: u32, message: &str| {
            progress_task.progress(current, total, message);
            Ok(())
        };
        let mut chunk = |content: &str, index: u32| {
            progress_task.chunk(content, index);
            Ok(())
        };

        run_generate_chapter_stream(
            state.inner(),
            user_id,
            GenerateChapterStreamInput {
                task_id: Some(task_id.clone()),
                ..input
            },
            Some(&mut progress),
            Some(&mut chunk),
            cancel_token,
        )
        .await
    };

    match result {
        Ok(done_task_id) => {
            progress_task.progress(scope.total, scope.total, "正文已保存");
            progress_task.complete(format!("第{chapter_number}章正文已保存"));
            progress_task.remove(state.inner());
            Ok(done_task_id)
        }
        Err(err) => {
            let cancelled = err == a4n_core::error::AppError::Cancelled.to_string();
            let code = if cancelled {
                "cancelled"
            } else {
                "generate_failed"
            };
            let message = if cancelled {
                "章节生成已取消"
            } else {
                err.as_str()
            };
            progress_task.error(code, message);
            progress_task.remove(state.inner());
            Err(err)
        }
    }
}
