//! 内容优化命令。
//!
//! Tauri 层只保留命令入口和桌面任务事件；优化会话、分析、补丁和对话流程
//! 统一由 `a4n-runtime::actions::optimize` 承载。

use a4n_runtime::actions::optimize::{
    AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
    CreateOptimizationChatSessionInput, OptimizationPatchSetSummary, OptimizationSessionSummary,
    OptimizationTurnSummary, PrepareOptimizationAnalysisSessionInput,
    ProposeOptimizationPatchesInput, ProposeOptimizationPatchesResponse,
    SendOptimizationMessageInput, SendOptimizationMessageResponse,
};

use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::error::AppError;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::actions::optimize as optimize_actions;
use tauri::{State, Window};

fn optimize_progress_reporter<'a>(
    progress: &'a WorkflowProgressTask,
    cancelled_message: &'static str,
) -> impl FnMut(u32, u32, &str) -> Result<(), AppError> + 'a {
    move |current, total, message| {
        progress.progress(current, total, message);
        progress
            .check_cancelled(cancelled_message)
            .map_err(AppError::InvalidArgument)
    }
}

fn optimize_stream_reporter<'a>(
    progress: &'a WorkflowProgressTask,
    cancelled_message: &'static str,
) -> impl FnMut(&str, u32) -> Result<(), AppError> + 'a {
    let mut stream_index = 0u32;
    move |delta, _index| {
        progress.chunk(delta, stream_index);
        stream_index = stream_index.saturating_add(1);
        progress
            .check_cancelled(cancelled_message)
            .map_err(AppError::InvalidArgument)
    }
}

macro_rules! optimize_session_command {
    ($name:ident, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            session_id: i64,
        ) -> Result<$output, String> {
            $action(state.inner(), user_id, session_id).await
        }
    };
}

macro_rules! optimize_input_command {
    ($name:ident, $input:ty, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<$output, String> {
            $action(state.inner(), user_id, input).await
        }
    };
}

macro_rules! optimize_session_stream_task_command {
    (
        $name:ident,
        $output:ty,
        $scope:path,
        $start_message:expr,
        $cancelled_message:expr,
        $error_code:expr,
        |$value:ident| $success_summary:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: Window,
            state: State<'_, AppState>,
            user_id: i64,
            session_id: i64,
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let scope = $scope(state.inner(), user_id, session_id).await?;
            let progress = WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let result = {
                let mut report_progress = optimize_progress_reporter(&progress, $cancelled_message);
                let mut report_stream = optimize_stream_reporter(&progress, $cancelled_message);
                $action(
                    state.inner(),
                    user_id,
                    session_id,
                    Some(&mut report_progress),
                    Some(&mut report_stream),
                )
                .await
            };
            progress.finish_result(state.inner(), &result, $error_code, |$value| {
                $success_summary
            });
            result
        }
    };
}

macro_rules! optimize_input_stream_task_command {
    (
        $name:ident,
        $input:ty,
        $output:ty,
        $scope:path,
        $start_message:expr,
        $cancelled_message:expr,
        $error_code:expr,
        |$value:ident| $success_summary:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: Window,
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let scope = $scope(state.inner(), user_id, &input).await?;
            let progress = WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let result = {
                let mut report_progress = optimize_progress_reporter(&progress, $cancelled_message);
                let mut report_stream = optimize_stream_reporter(&progress, $cancelled_message);
                $action(
                    state.inner(),
                    user_id,
                    input,
                    Some(&mut report_progress),
                    Some(&mut report_stream),
                )
                .await
            };
            progress.finish_result(state.inner(), &result, $error_code, |$value| {
                $success_summary
            });
            result
        }
    };
}

macro_rules! optimize_input_progress_task_command {
    (
        $name:ident,
        $input:ty,
        $output:ty,
        $scope:path,
        $start_message:expr,
        $cancelled_message:expr,
        $error_code:expr,
        |$value:ident| $success_summary:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            window: Window,
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
            task_id: Option<String>,
        ) -> Result<$output, String> {
            let scope = $scope(state.inner(), user_id, &input).await?;
            let progress = WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let result = {
                let mut report_progress = optimize_progress_reporter(&progress, $cancelled_message);
                $action(state.inner(), user_id, input, Some(&mut report_progress)).await
            };
            progress.finish_result(state.inner(), &result, $error_code, |$value| {
                $success_summary
            });
            result
        }
    };
}

macro_rules! optimize_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<ModelRequestPreview, String> {
            $action(state.inner(), user_id, input).await
        }
    };
}

#[tauri::command]
pub async fn list_optimize_sessions(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<OptimizationSessionSummary>, String> {
    optimize_actions::list_optimize_sessions(state.inner(), user_id, project_id, chapter_id).await
}

optimize_session_command!(
    get_optimize_session,
    OptimizationSessionSummary,
    optimize_actions::get_optimize_session,
);

optimize_session_command!(
    list_optimize_turns,
    Vec<OptimizationTurnSummary>,
    optimize_actions::list_optimize_turns,
);

optimize_session_command!(
    list_optimize_patch_sets,
    Vec<OptimizationPatchSetSummary>,
    optimize_actions::list_optimize_patch_sets,
);

optimize_session_command!(
    reject_pending_optimize_patch_sets,
    (),
    optimize_actions::reject_pending_optimize_patch_sets,
);

optimize_session_command!(
    delete_optimize_session,
    (),
    optimize_actions::delete_optimize_session,
);

#[tauri::command]
pub async fn record_optimize_agent_event(
    state: State<'_, AppState>,
    user_id: i64,
    session_id: i64,
    content: String,
) -> Result<(), String> {
    optimize_actions::record_optimize_agent_event(state.inner(), user_id, session_id, content).await
}

optimize_input_command!(
    create_optimization_chat_session,
    CreateOptimizationChatSessionInput,
    OptimizationSessionSummary,
    optimize_actions::create_optimization_chat_session,
);

optimize_input_command!(
    create_optimization_analysis_session,
    AnalyzeChapterInput,
    OptimizationSessionSummary,
    optimize_actions::create_optimization_analysis_session,
);

optimize_input_command!(
    prepare_optimization_analysis_session,
    PrepareOptimizationAnalysisSessionInput,
    OptimizationSessionSummary,
    optimize_actions::prepare_optimization_analysis_session,
);

optimize_session_stream_task_command!(
    execute_optimization_analysis_session,
    OptimizationSessionSummary,
    optimize_actions::execute_optimization_analysis_session_task_scope,
    "正在准备执行内容优化分析",
    "优化分析已停止",
    "optimize_analysis_failed",
    |session| optimization_analysis_success_summary(session),
    optimize_actions::execute_optimization_analysis_session,
);

optimize_preview_command!(
    preview_create_optimization_analysis_session_request,
    AnalyzeChapterInput,
    optimize_actions::preview_create_optimization_analysis_session_request,
);

optimize_preview_command!(
    preview_prepare_optimization_analysis_session_request,
    PrepareOptimizationAnalysisSessionInput,
    optimize_actions::preview_prepare_optimization_analysis_session_request,
);

optimize_input_stream_task_command!(
    propose_optimization_patches,
    ProposeOptimizationPatchesInput,
    ProposeOptimizationPatchesResponse,
    optimize_actions::propose_optimization_patches_task_scope,
    "正在准备局部修复补丁",
    "局部补丁生成已停止",
    "optimize_patch_failed",
    |response| format!("已生成 {} 个局部补丁", response.patches.len()),
    optimize_actions::propose_optimization_patches,
);

optimize_preview_command!(
    preview_propose_optimization_patches_request,
    ProposeOptimizationPatchesInput,
    optimize_actions::preview_propose_optimization_patches_request,
);

optimize_input_progress_task_command!(
    apply_optimization_patches,
    ApplyOptimizationPatchesInput,
    ApplyOptimizationPatchesResponse,
    optimize_actions::apply_optimization_patches_task_scope,
    "正在应用局部补丁",
    "应用局部补丁已停止",
    "optimize_patch_apply_failed",
    |response| format!(
        "已应用 {} 个局部补丁，字数 {}",
        response.applied_count, response.word_count
    ),
    optimize_actions::apply_optimization_patches,
);

optimize_input_stream_task_command!(
    send_optimization_message,
    SendOptimizationMessageInput,
    SendOptimizationMessageResponse,
    optimize_actions::send_optimization_message_task_scope,
    "正在准备内容优化 Agent 对话",
    "内容优化 Agent 对话已停止",
    "optimize_chat_failed",
    |_response| "内容优化 Agent 已回复",
    optimize_actions::send_optimization_message,
);

optimize_preview_command!(
    preview_send_optimization_message_request,
    SendOptimizationMessageInput,
    optimize_actions::preview_send_optimization_message_request,
);

fn optimization_analysis_success_summary(session: &OptimizationSessionSummary) -> String {
    let name = session
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty());
    match name {
        Some(name) => format!("优化分析完成：{name}"),
        None => "优化分析完成".to_string(),
    }
}
