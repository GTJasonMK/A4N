use a4n_db_models::models::ai::{optimization_session, optimization_turn};
use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use crate::optimize::OptimizationAgentPermissions;

pub(in crate::optimize) fn build_content_optimization_chat_user_message(
    user_message: &str,
    session: &optimization_session::Model,
    permissions: &OptimizationAgentPermissions,
    conversation_history: &Value,
    context_manifest: &Value,
) -> Result<String, AppError> {
    let user_message = user_message.trim();
    if user_message.is_empty() {
        return Err(AppError::InvalidArgument("对话消息不能为空".to_string()));
    }

    let payload = json!({
        "task": {
            "name": "content_optimization_chat_agent_loop",
            "requirement": "围绕 user_message 完成本轮对话；必要时读取上下文或提出补丁，不扩大为整章分析。",
        },
        "automatic_writeback": permissions.automatic_writeback,
        "context_manifest": context_manifest,
        "conversation_history": conversation_history,
        "session": {
            "issues": session.issues,
            "summary": session.summary,
        },
        "user_message": user_message,
    });
    build_json_task_user_message(
        "继续内容优化对话 Agent Loop",
        "遵守 system 的决策协议，只返回一个严格 JSON 决策对象",
        &payload,
    )
}

pub(in crate::optimize) fn optimization_chat_history(turns: &[optimization_turn::Model]) -> Value {
    Value::Array(
        turns
            .iter()
            .map(|turn| {
                json!({
                    "status": turn.status,
                    "user": turn.user_text,
                    "assistant": turn.final_reply,
                })
            })
            .collect(),
    )
}
