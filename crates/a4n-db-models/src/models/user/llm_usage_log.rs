//! LLM 使用日志实体【新增】

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_usage_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    pub project_id: Option<i64>,

    /// 调用的模型
    pub model: String,

    /// 操作类型：chat / stream / embed / analyze / generate_image
    pub operation: String,

    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,

    /// 响应耗时（毫秒）
    pub latency_ms: i32,

    /// 是否成功
    pub is_success: bool,

    /// 错误信息
    pub error_message: Option<String>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
