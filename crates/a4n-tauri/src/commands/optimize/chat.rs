use a4n_runtime::host_api::optimize::{
    self as optimize_actions, CreateOptimizationChatSessionInput, OptimizationSessionSummary,
    SendOptimizationMessageInput, SendOptimizationMessageResponse,
};

use super::adapters::{optimize_input_command, optimize_preview_command};
use super::task::optimize_input_stream_task_command;

optimize_input_command!(
    create_optimization_chat_session,
    CreateOptimizationChatSessionInput,
    OptimizationSessionSummary,
    optimize_actions::create_optimization_chat_session,
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
