//! LLM 调用成本与上下文追踪实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "llm_call_trace")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub generation_run_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub context_plan_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub context_pack_refs_json: Option<serde_json::Value>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub prompt_cache_hit_tokens: Option<i32>,
    pub prompt_cache_miss_tokens: Option<i32>,
    pub prompt_cache_write_tokens: Option<i32>,
    pub prompt_cache_hit_ratio: Option<f64>,
    pub duration_ms: Option<i64>,
    pub streamed: bool,
    pub status: String,
    pub error_kind: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub error_message: Option<String>,
    pub finish_reason: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::generation_run::Entity",
        from = "Column::GenerationRunId",
        to = "super::generation_run::Column::Id"
    )]
    GenerationRun,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::generation_run::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GenerationRun.def()
    }
}
