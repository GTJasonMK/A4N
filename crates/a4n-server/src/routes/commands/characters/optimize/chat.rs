use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, TaskType, command_input, finish_web_task, optimize_actions,
    optional_string_field, register_web_task, sanitize_optional_task_id, string_action_result,
    to_value, web_chunk_reporter, web_progress_reporter,
};
use super::adapters::optimize_web_input_command;
use super::task::{optimize_web_input_stream_task_command, optimize_web_preview_command};

optimize_web_input_command!(
    create_optimization_chat_session,
    optimize_actions::CreateOptimizationChatSessionInput,
    optimize_actions::create_optimization_chat_session,
);
optimize_web_input_stream_task_command!(
    send_optimization_message,
    optimize_actions::SendOptimizationMessageInput,
    optimize_actions::send_optimization_message_task_scope,
    "正在准备内容优化 Agent 对话",
    "内容优化 Agent 已回复",
    optimize_actions::send_optimization_message,
);
optimize_web_preview_command!(
    preview_send_optimization_message_request,
    optimize_actions::SendOptimizationMessageInput,
    optimize_actions::preview_send_optimization_message_request,
);
