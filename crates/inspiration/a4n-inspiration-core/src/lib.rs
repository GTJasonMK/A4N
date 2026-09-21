//! Pure inspiration-history projections.

use a4n_api_contract::inspiration::InspirationMessageSummary;
use a4n_db_models::models::user::message;

pub fn message_summary(message: message::Model) -> InspirationMessageSummary {
    InspirationMessageSummary {
        id: message.id,
        role: message.role,
        content: message.content,
        created_at: message.created_at.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_projection_keeps_history_fields() {
        let summary = message_summary(message::Model {
            id: 7,
            conversation_id: 3,
            role: "assistant".into(),
            content: "内容".into(),
            tool_calls: None,
            prompt_tokens: None,
            completion_tokens: None,
            chapter_number: None,
            is_important: false,
            created_at: Default::default(),
        });
        assert_eq!(summary.id, 7);
        assert_eq!(summary.role, "assistant");
        assert_eq!(summary.content, "内容");
    }
}
