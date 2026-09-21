//! 模型生成运行记录实体。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "generation_run")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub workflow_run_id: Option<i64>,
    pub call_key: Option<String>,
    pub input_fingerprint_sha256: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub input_fingerprint_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub raw_response_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub parsed_output_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub validation_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub artifact_refs_json: Option<serde_json::Value>,
    pub attempt: i32,
    #[sea_orm(column_type = "Json")]
    pub prompt_template_names_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub prompt_template_hashes_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub rendered_messages_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub rendered_prompts_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub input_variables_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub source_refs_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub media_refs_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub request_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json")]
    pub response_summary_json: Option<serde_json::Value>,
    #[sea_orm(column_type = "Text")]
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
