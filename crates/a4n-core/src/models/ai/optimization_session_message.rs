//! 优化会话消息实体【新增】

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "optimization_session_message")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub session_id: i64,

    pub role: String,

    #[sea_orm(column_type = "Text")]
    pub content: String,

    /// 工具调用记录
    #[sea_orm(column_type = "Json")]
    pub tool_calls: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::optimization_session::Entity",
        from = "Column::SessionId",
        to = "super::optimization_session::Column::Id"
    )]
    OptimizationSession,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::optimization_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OptimizationSession.def()
    }
}
