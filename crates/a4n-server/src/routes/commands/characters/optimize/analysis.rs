use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, optimize_actions, strict_runtime_command_input, to_value,
};
use super::adapters::optimize_web_runtime_input_command;

optimize_web_runtime_input_command!(
    create_optimization_analysis_session,
    optimize_actions::AnalyzeChapterInput,
    optimize_actions::create_optimization_analysis_session,
);
optimize_web_runtime_input_command!(
    prepare_optimization_analysis_session,
    optimize_actions::PrepareOptimizationAnalysisSessionInput,
    optimize_actions::prepare_optimization_analysis_session,
);
optimize_web_runtime_input_command!(
    preview_create_optimization_analysis_session_request,
    optimize_actions::AnalyzeChapterInput,
    optimize_actions::preview_create_optimization_analysis_session_request,
);
optimize_web_runtime_input_command!(
    preview_prepare_optimization_analysis_session_request,
    optimize_actions::PrepareOptimizationAnalysisSessionInput,
    optimize_actions::preview_prepare_optimization_analysis_session_request,
);
