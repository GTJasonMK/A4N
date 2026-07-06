//! 大纲系统命令。
//!
//! Tauri 层只负责命令签名和窗口进度；大纲业务逻辑统一在
//! `a4n_runtime::actions::outline_records`。

use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::actions::outline_records as outline_actions;
use outline_actions::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ChapterOutlineSummary,
    ContinuePartOutlinesInput, GenerateChapterOutlinesByCountInput,
    GeneratePartChapterOutlinesInput, GeneratePartsInput, GenerationProgressSummary,
    PartOutlineSummary, RegenerateChapterOutlineInput, RegenerateLastPartInput,
    RegeneratePartOutlineInput, RegeneratePartsInput, UpdateChapterOutlineInput,
};
use tauri::State;

const OUTLINE_CANCELLED_MESSAGE: &str = "大纲生成已停止";

fn progress_reporter<'a>(
    progress: &'a WorkflowProgressTask,
) -> impl Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'a {
    move |current, total, message| {
        progress.progress(current, total, message);
        progress
            .check_cancelled(OUTLINE_CANCELLED_MESSAGE)
            .map_err(|_| AppError::Cancelled)
    }
}

macro_rules! run_progress_task {
    (
        $state:expr,
        $window:expr,
        $task_id:expr,
        $project_id:expr,
        $total:expr,
        $start_message:expr,
        $error_code:expr,
        $success_message:expr,
        |$progress:ident| $run:expr $(,)?
    ) => {{
        async {
            let progress_task = WorkflowProgressTask::new(
                $window,
                $state.inner(),
                $task_id,
                TaskType::LlmCall,
                Some($project_id),
                $total,
                $start_message,
            );
            let result = {
                let reporter = progress_reporter(&progress_task);
                let $progress: Option<outline_actions::OutlineProgressReporter<'_>> =
                    Some(&reporter);
                $run.await.map_err(|err| err.to_string())
            };
            progress_task.finish_result($state.inner(), &result, $error_code, $success_message);
            result
        }
    }};
}

macro_rules! outline_generation_command {
    (
        $name:ident,
        $input:ty,
        $output:ty,
        $scope:path,
        |$message_input:ident| $start_message:expr,
        $error_code:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            window: tauri::Window,
            input: $input,
        ) -> Result<$output, String> {
            let task_id = input.task_id.clone();
            let scope = $scope(state.inner(), user_id, &input)
                .await
                .map_err(|e| e.to_string())?;
            let start_message = {
                let $message_input = &input;
                ($start_message).to_string()
            };
            run_progress_task!(
                state,
                window,
                task_id,
                scope.project_id,
                scope.total,
                &start_message,
                $error_code,
                |_| $success_message,
                |progress| $action(state.inner(), user_id, input, progress),
            )
            .await
        }
    };
}

macro_rules! outline_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<ModelRequestPreview, String> {
            $action(state.inner(), user_id, input)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

#[tauri::command]
pub async fn list_parts(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PartOutlineSummary>, String> {
    outline_actions::list_parts(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

outline_generation_command!(
    generate_parts,
    GeneratePartsInput,
    Vec<PartOutlineSummary>,
    outline_actions::generate_parts_task_scope,
    |_input| "启动分部大纲生成",
    "outline_generate_failed",
    "分部大纲已生成",
    outline_actions::generate_parts,
);

outline_preview_command!(
    preview_generate_parts_request,
    GeneratePartsInput,
    outline_actions::preview_generate_parts_request,
);

#[tauri::command]
pub async fn delete_latest_parts(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<(), String> {
    outline_actions::delete_latest_parts(state.inner(), user_id, project_id, count)
        .await
        .map_err(|e| e.to_string())
}

outline_generation_command!(
    regenerate_parts,
    RegeneratePartsInput,
    Vec<PartOutlineSummary>,
    outline_actions::regenerate_parts_task_scope,
    |_input| "启动分部大纲重生成",
    "outline_regenerate_failed",
    "已重新生成所有部",
    outline_actions::regenerate_parts,
);

outline_preview_command!(
    preview_regenerate_parts_request,
    RegeneratePartsInput,
    outline_actions::preview_regenerate_parts_request,
);

outline_generation_command!(
    regenerate_last_part,
    RegenerateLastPartInput,
    PartOutlineSummary,
    outline_actions::regenerate_last_part_task_scope,
    |_input| "正在重新生成最后一部",
    "outline_regenerate_last_failed",
    "最后一部已重新生成",
    outline_actions::regenerate_last_part,
);

outline_preview_command!(
    preview_regenerate_last_part_request,
    RegenerateLastPartInput,
    outline_actions::preview_regenerate_last_part_request,
);

#[tauri::command]
pub async fn get_generation_progress(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<GenerationProgressSummary, String> {
    outline_actions::get_generation_progress(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_chapter_outlines(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterOutlineSummary>, String> {
    outline_actions::list_chapter_outlines(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_latest_chapter_outlines(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<(), String> {
    outline_actions::delete_latest_chapter_outlines(state.inner(), user_id, project_id, count)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_chapter_outline(
    state: State<'_, AppState>,
    user_id: i64,
    input: UpdateChapterOutlineInput,
) -> Result<ChapterOutlineSummary, String> {
    outline_actions::update_chapter_outline(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

outline_generation_command!(
    regenerate_chapter_outline,
    RegenerateChapterOutlineInput,
    ChapterOutlineSummary,
    outline_actions::regenerate_chapter_outline_task_scope,
    |_input| "启动章节大纲重生成",
    "outline_chapter_regenerate_failed",
    "章节大纲已重新生成",
    outline_actions::regenerate_chapter_outline,
);

outline_preview_command!(
    preview_regenerate_chapter_outline_request,
    RegenerateChapterOutlineInput,
    outline_actions::preview_regenerate_chapter_outline_request,
);

outline_generation_command!(
    continue_part_outlines,
    ContinuePartOutlinesInput,
    PartOutlineSummary,
    outline_actions::continue_part_outlines_task_scope,
    |input| format!("正在续作第 {} 部", input.part_number),
    "outline_continue_part_failed",
    "分部续作完成",
    outline_actions::continue_part_outlines,
);

outline_preview_command!(
    preview_continue_part_outlines_request,
    ContinuePartOutlinesInput,
    outline_actions::preview_continue_part_outlines_request,
);

outline_generation_command!(
    regenerate_part_outline,
    RegeneratePartOutlineInput,
    PartOutlineSummary,
    outline_actions::regenerate_part_outline_task_scope,
    |_input| "正在重新生成指定分部",
    "outline_part_regenerate_failed",
    "指定分部已重新生成",
    outline_actions::regenerate_part_outline,
);

outline_preview_command!(
    preview_regenerate_part_outline_request,
    RegeneratePartOutlineInput,
    outline_actions::preview_regenerate_part_outline_request,
);

outline_generation_command!(
    generate_part_chapter_outlines,
    GeneratePartChapterOutlinesInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::generate_part_chapter_outlines_task_scope,
    |_input| "启动本部章节大纲生成",
    "outline_part_chapters_generate_failed",
    "本部章节大纲已生成",
    outline_actions::generate_part_chapter_outlines,
);

outline_preview_command!(
    preview_generate_part_chapter_outlines_request,
    GeneratePartChapterOutlinesInput,
    outline_actions::preview_generate_part_chapter_outlines_request,
);

outline_generation_command!(
    batch_generate_part_outlines,
    BatchGeneratePartOutlinesInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::batch_generate_part_outlines_task_scope,
    |_input| "启动批量章节大纲生成",
    "outline_batch_chapters_generate_failed",
    "批量章节大纲已生成",
    outline_actions::batch_generate_part_outlines,
);

outline_preview_command!(
    preview_batch_generate_part_outlines_request,
    BatchGeneratePartOutlinesInput,
    outline_actions::preview_batch_generate_part_outlines_request,
);

outline_generation_command!(
    apply_beat_mapping,
    ApplyBeatMappingInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::apply_beat_mapping_task_scope,
    |_input| "启动节拍映射",
    "outline_beat_mapping_failed",
    "节拍映射已应用",
    outline_actions::apply_beat_mapping,
);

outline_preview_command!(
    preview_apply_beat_mapping_request,
    ApplyBeatMappingInput,
    outline_actions::preview_apply_beat_mapping_request,
);

outline_generation_command!(
    generate_chapter_outlines_by_count,
    GenerateChapterOutlinesByCountInput,
    Vec<ChapterOutlineSummary>,
    outline_actions::generate_chapter_outlines_by_count_task_scope,
    |_input| "启动章节大纲追加生成",
    "outline_chapters_generate_failed",
    "章节大纲已生成",
    outline_actions::generate_chapter_outlines_by_count,
);

outline_preview_command!(
    preview_generate_chapter_outlines_by_count_request,
    GenerateChapterOutlinesByCountInput,
    outline_actions::preview_generate_chapter_outlines_by_count_request,
);

#[tauri::command]
pub async fn get_chapter_outlines_pacing(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    outline_actions::get_chapter_outlines_pacing(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}
