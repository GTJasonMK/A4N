//! Pure inspiration-history projections.

use a4n_api_contract::inspiration::InspirationMessageSummary;
use a4n_domain::inspiration::InspirationMessageRecord;

pub fn message_summary(
    message: InspirationMessageRecord,
) -> Result<InspirationMessageSummary, String> {
    if message.role == "assistant" {
        a4n_domain::inspiration::parse_inspiration_response(&message.content)
            .map_err(|error| format!("灵感历史消息 {} 无效: {error}", message.id))?;
    }
    Ok(InspirationMessageSummary {
        id: message.id,
        role: message.role,
        content: message.content,
        created_at: message.created_at.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_projection_keeps_user_fields_and_rejects_invalid_assistant_history() {
        let message = InspirationMessageRecord {
            id: 7,
            role: "user".into(),
            content: "内容".into(),
            created_at: Default::default(),
        };
        let mut invalid = message.clone();
        invalid.role = "assistant".into();
        assert!(
            message_summary(invalid)
                .unwrap_err()
                .contains("灵感历史消息 7 无效")
        );
        let summary = message_summary(message).unwrap();
        assert_eq!(summary.id, 7);
        assert_eq!(summary.role, "user");
        assert_eq!(summary.content, "内容");
    }
}
