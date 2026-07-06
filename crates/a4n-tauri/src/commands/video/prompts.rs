use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::{WorkflowProgressTask, WorkflowTaskContext};
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::actions::dependency_conflict::DownstreamConflictStrategy;
use a4n_runtime::actions::video_prompts::{
    self as video_prompt_actions, ChapterVideoPromptSummary, DeleteChapterVideoPromptGroupResult,
    GeneratePromptOptionsInput, OptimizeChapterVideoPromptInput, UpdateChapterVideoPromptInput,
};
use tauri::{State, Window};

macro_rules! tauri_video_prompt_command {
    (
        $name:ident,
        $start_message:expr,
        $stop_message:expr,
        $error_code:expr,
        $success_summary:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: Window,
            state: State<'_, AppState>,
            user_id: i64,
            project_id: i64,
            chapter_id: i64,
            options: GeneratePromptOptionsInput,
            task_id: Option<String>,
        ) -> Result<Vec<ChapterVideoPromptSummary>, String> {
            let input = video_prompt_input(user_id, project_id, chapter_id, options);
            let scope =
                video_prompt_actions::chapter_video_prompt_task_scope(state.inner(), &input)
                    .await?;
            let progress =
                video_prompt_progress_task(window, state.inner(), task_id, scope, $start_message);
            let mut report_progress = tauri_video_prompt_progress(&progress, $stop_message);
            let mut report_chunk = tauri_video_prompt_chunk(&progress);
            let result = $action(
                state.inner(),
                input,
                Some(&mut report_progress),
                Some(&mut report_chunk),
                progress.cancel_token(),
            )
            .await;
            drop(report_chunk);
            drop(report_progress);
            progress.finish_result(state.inner(), &result, $error_code, |rows| {
                ($success_summary)(rows.as_slice())
            });
            result
        }
    };
}

tauri_video_prompt_command!(
    generate_chapter_video_shot_analysis,
    "正在准备镜头分析生成",
    "镜头分析生成已停止",
    "video_shot_analysis_generation_failed",
    |rows: &[ChapterVideoPromptSummary]| format!("已生成 {} 条镜头分析记录", rows.len()),
    video_prompt_actions::generate_chapter_video_shot_analysis,
);

tauri_video_prompt_command!(
    generate_chapter_video_segment_plan,
    "正在准备片段规划生成",
    "片段规划生成已停止",
    "video_segment_plan_generation_failed",
    |rows: &[ChapterVideoPromptSummary]| format!("已生成 {} 条片段规划记录", rows.len()),
    video_prompt_actions::generate_chapter_video_segment_plan,
);

tauri_video_prompt_command!(
    generate_chapter_video_frame_prompts,
    "正在准备边界帧提示词生成",
    "边界帧提示词生成已停止",
    "video_frame_prompt_generation_failed",
    frame_prompt_generation_success_summary,
    video_prompt_actions::generate_chapter_video_frame_prompts,
);

tauri_video_prompt_command!(
    generate_chapter_video_prompts,
    "正在准备视频提示词生成",
    "视频提示词生成已停止",
    "video_prompt_generation_failed",
    |rows: &[ChapterVideoPromptSummary]| format!("已生成 {} 条视频提示词记录", rows.len()),
    video_prompt_actions::generate_chapter_video_prompts,
);

fn frame_prompt_generation_success_summary(rows: &[ChapterVideoPromptSummary]) -> String {
    format!(
        "已生成 {} 条帧提示词记录",
        generated_frame_prompt_record_count(rows)
    )
}

fn generated_frame_prompt_record_count(rows: &[ChapterVideoPromptSummary]) -> usize {
    rows.iter()
        .filter(|row| is_generated_frame_prompt_type(&row.prompt_type))
        .count()
}

fn is_generated_frame_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_prompt_success_count_excludes_upstream_records() {
        let prompt_types = [
            "frame_shot_analysis",
            "frame_segment_plan",
            "frame_prompt",
            "first_frame_shot_analysis",
            "first_frame_segment_plan",
            "first_frame_prompt",
            "chat_video_shot_analysis",
            "chat_video_segment_plan",
            "chat_video_frame_prompt",
            "video_prompt",
        ];
        let count = prompt_types
            .iter()
            .filter(|prompt_type| is_generated_frame_prompt_type(prompt_type))
            .count();

        assert_eq!(count, 3);
    }
}

#[tauri::command]
pub async fn preview_chapter_video_frame_prompt_requests(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    options: GeneratePromptOptionsInput,
) -> Result<ModelRequestPreview, String> {
    video_prompt_actions::preview_chapter_video_frame_prompt_model_request(
        state.inner(),
        video_prompt_input(user_id, project_id, chapter_id, options),
    )
    .await
}

#[tauri::command]
pub async fn preview_chapter_video_prompt_requests(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    options: GeneratePromptOptionsInput,
) -> Result<ModelRequestPreview, String> {
    video_prompt_actions::preview_chapter_video_prompt_model_request(
        state.inner(),
        video_prompt_input(user_id, project_id, chapter_id, options),
    )
    .await
}

#[tauri::command]
pub async fn list_chapter_video_prompts(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    video_prompt_actions::list_chapter_video_prompts(state.inner(), user_id, project_id, chapter_id)
        .await
}

#[tauri::command]
pub async fn update_chapter_video_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    input: UpdateChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    video_prompt_actions::update_chapter_video_prompt(state.inner(), user_id, prompt_id, input)
        .await
}

#[tauri::command]
pub async fn optimize_chapter_video_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    input: OptimizeChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    video_prompt_actions::optimize_chapter_video_prompt(state.inner(), user_id, prompt_id, input)
        .await
}

#[tauri::command]
pub async fn delete_chapter_video_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: Option<String>,
) -> Result<(), String> {
    let downstream_strategy =
        DownstreamConflictStrategy::from_optional_str(downstream_strategy.as_deref())?;
    video_prompt_actions::delete_chapter_video_prompt(
        state.inner(),
        user_id,
        prompt_id,
        downstream_strategy,
    )
    .await
}

#[tauri::command]
pub async fn delete_chapter_video_prompt_group(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    group: String,
    downstream_strategy: Option<String>,
) -> Result<DeleteChapterVideoPromptGroupResult, String> {
    let downstream_strategy =
        DownstreamConflictStrategy::from_optional_str(downstream_strategy.as_deref())?;
    video_prompt_actions::delete_chapter_video_prompt_group(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        &group,
        downstream_strategy,
    )
    .await
}

fn video_prompt_progress_task(
    window: Window,
    state: &AppState,
    task_id: Option<String>,
    scope: video_prompt_actions::VideoPromptTaskScope,
    message: &str,
) -> WorkflowProgressTask {
    WorkflowProgressTask::new_with_context(
        window,
        state,
        task_id,
        TaskType::LlmCall,
        WorkflowTaskContext {
            project_id: Some(scope.project_id),
            root_project_id: Some(scope.project_id),
            source_snapshot_id: None,
            task_scope: Some("video_prompt".to_string()),
        },
        scope.total,
        message,
    )
}

fn tauri_video_prompt_progress<'a>(
    progress: &'a WorkflowProgressTask,
    stop_message: &'static str,
) -> impl FnMut(u32, u32, &str) -> Result<(), AppError> + Send + 'a {
    move |current, total, message| {
        progress.progress(current, total, message);
        progress
            .check_cancelled(stop_message)
            .map_err(|_| AppError::Cancelled)
    }
}

fn tauri_video_prompt_chunk<'a>(
    progress: &'a WorkflowProgressTask,
) -> impl FnMut(&str, u32) + Send + 'a {
    let mut stream_index = 0_u32;
    move |content, _index| {
        progress.chunk(content, stream_index);
        stream_index = stream_index.saturating_add(1);
    }
}

fn video_prompt_input(
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    options: GeneratePromptOptionsInput,
) -> video_prompt_actions::GenerateChapterVideoPromptInput {
    video_prompt_actions::GenerateChapterVideoPromptInput {
        user_id,
        project_id,
        chapter_id,
        options,
    }
}
