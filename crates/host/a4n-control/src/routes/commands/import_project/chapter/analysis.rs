use crate::routes::commands::{
    ApiError, command_input, finish_web_task, optional_string_field, register_web_task,
    sanitize_optional_task_id, to_value, web_fire_and_forget_progress_reporter,
    web_stream_reporter,
};
use crate::state::ServerState;
use a4n_api_contract::chapter::{CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisInput};
use a4n_domain::task_queue::TaskKind;
use serde_json::Value;

macro_rules! chapter_analysis_web_task_command {
    (
        $name:ident,
        $command_name:expr,
        $preflight:path,
        $run:path,
        $start_message:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: ChapterAnalysisInput = command_input(args, $command_name)?;
            let ch = $preflight(state.runtime(), user_id, input.chapter_id).await?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                user_id,
                task_id.clone(),
                TaskKind::LlmCall,
                ch.project_id,
                CHAPTER_ANALYSIS_PROGRESS_TOTAL,
                $start_message,
            )?;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter(task.clone());
            // 与桌面端对称：活体信号必须双端一致。
            let stream_reporter = web_stream_reporter(task.clone());
            let progress = a4n_runtime::host_api::chapter_analysis::ChapterAnalysisProgress::new(
                Some(&reporter),
                cancel_token.as_ref(),
                task_id.as_deref(),
            )
            .with_stream_reporter(Some(&stream_reporter));
            let result = $run(state.runtime(), user_id, input.chapter_id, Some(&progress)).await;
            let value = finish_web_task(state, task.as_ref(), result)?;
            to_value(value)
        }
    };
}

macro_rules! chapter_analysis_web_preview_command {
    ($name:ident, $command_name:expr, $preview:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: ChapterAnalysisInput = command_input(args, $command_name)?;
            let preview = $preview(state.runtime(), user_id, input.chapter_id).await?;
            to_value(preview)
        }
    };
}

chapter_analysis_web_task_command!(
    generate_chapter_analysis,
    "generate_chapter_analysis",
    a4n_runtime::host_api::chapter_analysis::preflight_generate_chapter_analysis,
    a4n_runtime::host_api::chapter_analysis::generate_chapter_analysis_with_progress,
    "正在准备章节结构化分析",
);

chapter_analysis_web_preview_command!(
    preview_chapter_analysis_request,
    "preview_chapter_analysis_request",
    a4n_runtime::host_api::chapter_analysis::preview_chapter_analysis_request,
);

chapter_analysis_web_task_command!(
    resume_chapter_analysis_sync,
    "resume_chapter_analysis_sync",
    a4n_runtime::host_api::chapter_analysis::preflight_resume_chapter_analysis_sync,
    a4n_runtime::host_api::chapter_analysis::resume_chapter_analysis_sync_with_progress,
    "正在准备继续同步章节分析",
);

chapter_analysis_web_preview_command!(
    preview_resume_chapter_analysis_sync_request,
    "preview_resume_chapter_analysis_sync_request",
    a4n_runtime::host_api::chapter_analysis::preview_resume_chapter_analysis_sync_request,
);
