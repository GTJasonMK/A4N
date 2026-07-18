use a4n_runtime::host_api::optimize::{
    self as optimize_actions, AnalyzeChapterInput, OptimizationSessionSummary,
    PrepareOptimizationAnalysisSessionInput,
};

use super::adapters::{optimize_input_command, optimize_preview_command};
use super::task::optimize_session_stream_task_command;

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
