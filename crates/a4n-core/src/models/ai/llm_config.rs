//! LLM 配置实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 配置名称（可定义多组 LLM 配置）
    pub name: String,

    /// Provider 类型：openai / ollama / anthropic
    pub provider: String,

    /// API Base URL
    pub base_url: String,

    /// API Key（加密存储）
    pub api_key_encrypted: Option<String>,

    /// 默认模型
    pub model: String,

    /// 最大并发数
    pub max_concurrent: i32,

    /// 请求超时（秒）
    pub timeout_secs: i32,

    /// 模型总上下文窗口（输入 + 输出）。
    pub max_context_tokens: i32,

    /// 模型单次最大输出 token 数。
    pub max_output_tokens: i32,

    /// Provider-specific extra JSON fields merged into chat/completions requests.
    pub request_options_json: Option<Value>,

    /// 是否设为默认
    pub is_default: bool,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
