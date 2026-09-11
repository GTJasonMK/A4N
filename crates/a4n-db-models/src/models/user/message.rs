//! 对话消息实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "message")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub conversation_id: i64,

    /// 角色：user / assistant / system / tool
    pub role: String,

    /// 消息内容
    #[sea_orm(column_type = "Text")]
    pub content: String,

    /// 工具调用（JSON）
    #[sea_orm(column_type = "Json")]
    pub tool_calls: Option<serde_json::Value>,

    /// Token 用量
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,

    /// 关联章节
    pub chapter_number: Option<i32>,

    /// 是否被标记为重要
    pub is_important: bool,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::conversation::Entity",
        from = "Column::ConversationId",
        to = "super::conversation::Column::Id"
    )]
    Conversation,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::conversation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Conversation.def()
    }
}
