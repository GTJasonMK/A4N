use serde_json::{Value, json};

use super::super::super::{ApiError, ServerState, command_input, optimize_actions, to_value};
use super::adapters::optimize_web_input_command;
use super::adapters::optimize_web_preview_command;

optimize_web_input_command!(
    create_optimization_chat_session,
    optimize_actions::CreateOptimizationChatSessionInput,
    optimize_actions::create_optimization_chat_session,
);
optimize_web_preview_command!(
    preview_send_optimization_message_request,
    optimize_actions::SendOptimizationMessageInput,
    optimize_actions::preview_send_optimization_message_request,
);
