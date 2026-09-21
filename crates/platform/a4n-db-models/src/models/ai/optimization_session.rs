//! 内容优化会话实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "optimization_session")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub name: Option<String>,

    /// 会话状态：analyzing / waiting_user / fixing / completed / cancelled
    pub status: String,

    /// 分析阶段发现的问题列表（JSON）
    #[sea_orm(column_type = "Json")]
    pub issues: Option<serde_json::Value>,

    /// 本次 Agent 权限配置
    #[sea_orm(column_type = "Json")]
    pub agent_permissions: Option<serde_json::Value>,

    /// 本次用户给 Agent 的额外指令
    #[sea_orm(column_type = "Text")]
    pub agent_instruction: Option<String>,

    /// 已应用的修复数量
    pub fixes_applied: i32,

    /// 会话摘要
    #[sea_orm(column_type = "Text")]
    pub summary: Option<String>,

    /// 旧版用户侧优化会话归档时间；归档记录保留供历史任务使用。
    pub archived_at: Option<chrono::NaiveDateTime>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::optimization_session_message::Entity")]
    Messages,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::optimization_session_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Messages.def()
    }
}
