use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, optimize_actions, strict_runtime_command_input, to_value,
};
use super::adapters::optimize_web_runtime_input_command;

optimize_web_runtime_input_command!(
    create_optimization_chat_session,
    optimize_actions::CreateOptimizationChatSessionInput,
    optimize_actions::create_optimization_chat_session,
);
optimize_web_runtime_input_command!(
    preview_send_optimization_message_request,
    optimize_actions::SendOptimizationMessageInput,
    optimize_actions::preview_send_optimization_message_request,
);
