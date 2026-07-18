use super::contracts::MessageSummary;
use a4n_core::models::user::message;

pub(super) fn message_summary(message: &message::Model) -> MessageSummary {
    MessageSummary {
        id: message.id,
        conversation_id: message.conversation_id,
        role: message.role.clone(),
        content: message.content.clone(),
        tool_calls: message.tool_calls.clone(),
        prompt_tokens: message.prompt_tokens,
        completion_tokens: message.completion_tokens,
        chapter_number: message.chapter_number,
        is_important: message.is_important,
        created_at: Some(message.created_at.to_string()),
    }
}
