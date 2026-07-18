use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, TaskType, command_input, finish_web_task, i64_arg, optimize_actions,
    optional_string_field, register_web_task, sanitize_optional_task_id, string_action_result,
    to_value, web_chunk_reporter, web_progress_reporter,
};
use super::adapters::optimize_web_input_command;
use super::task::{optimize_web_preview_command, optimize_web_session_stream_task_command};

optimize_web_input_command!(
    create_optimization_analysis_session,
    optimize_actions::AnalyzeChapterInput,
    optimize_actions::create_optimization_analysis_session,
);
optimize_web_input_command!(
    prepare_optimization_analysis_session,
    optimize_actions::PrepareOptimizationAnalysisSessionInput,
    optimize_actions::prepare_optimization_analysis_session,
);
optimize_web_session_stream_task_command!(
    execute_optimization_analysis_session,
    optimize_actions::execute_optimization_analysis_session_task_scope,
    "正在准备执行内容优化分析",
    "优化分析完成",
    optimize_actions::execute_optimization_analysis_session,
);
optimize_web_preview_command!(
    preview_create_optimization_analysis_session_request,
    optimize_actions::AnalyzeChapterInput,
    optimize_actions::preview_create_optimization_analysis_session_request,
);
optimize_web_preview_command!(
    preview_prepare_optimization_analysis_session_request,
    optimize_actions::PrepareOptimizationAnalysisSessionInput,
    optimize_actions::preview_prepare_optimization_analysis_session_request,
);
