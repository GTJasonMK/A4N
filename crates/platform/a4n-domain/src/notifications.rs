use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_NOTIFICATION_TITLE_CHARS: usize = 120;
pub const MAX_NOTIFICATION_BODY_CHARS: usize = 5_000;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BroadcastNotificationInput {
    pub title: String,
    pub body: String,
}

impl BroadcastNotificationInput {
    pub fn normalize_and_validate(mut self) -> Result<Self, String> {
        self.title = self.title.trim().to_string();
        self.body = self.body.trim().to_string();

        if self.title.is_empty() {
            return Err("通知标题不能为空".into());
        }
        if self.title.chars().count() > MAX_NOTIFICATION_TITLE_CHARS {
            return Err(format!(
                "通知标题不能超过 {MAX_NOTIFICATION_TITLE_CHARS} 个字符"
            ));
        }
        if self.body.is_empty() {
            return Err("通知内容不能为空".into());
        }
        if self.body.chars().count() > MAX_NOTIFICATION_BODY_CHARS {
            return Err(format!(
                "通知内容不能超过 {MAX_NOTIFICATION_BODY_CHARS} 个字符"
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BroadcastNotificationReceipt {
    pub notification_id: i64,
    pub recipient_count: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CreatorNotificationItem {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CreatorNotificationInbox {
    pub items: Vec<CreatorNotificationItem>,
    pub unread_count: u64,
}
