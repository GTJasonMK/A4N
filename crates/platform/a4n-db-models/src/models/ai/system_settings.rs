//! 系统设置实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const PROMPT_OVERRIDE_CATEGORY: &str = "prompt_override";
pub const PROMPT_OVERRIDE_KEY_PREFIX: &str = "prompt_override_";
pub const PROMPT_ASSEMBLY_KEY_PREFIX: &str = "prompt_assembly_";

pub fn prompt_override_key_prefix(project_id: i64) -> String {
    format!("{PROMPT_OVERRIDE_KEY_PREFIX}{project_id}_")
}

pub fn prompt_override_key(project_id: i64, name: &str) -> String {
    format!("{}{}", prompt_override_key_prefix(project_id), name)
}

pub fn prompt_assembly_global_key(call_key: &str, prompt_name: &str) -> String {
    format!("{PROMPT_ASSEMBLY_KEY_PREFIX}global_{call_key}::{prompt_name}")
}

pub fn prompt_assembly_project_key(project_id: i64, call_key: &str, prompt_name: &str) -> String {
    format!("{PROMPT_ASSEMBLY_KEY_PREFIX}project_{project_id}_{call_key}::{prompt_name}")
}

pub fn prompt_assembly_project_prefix(project_id: i64) -> String {
    format!("{PROMPT_ASSEMBLY_KEY_PREFIX}project_{project_id}_")
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "system_settings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 设置键（唯一）
    pub key: String,

    /// 设置值（JSON）
    #[sea_orm(column_type = "Json")]
    pub value: serde_json::Value,

    /// 分类（general/llm/export/ui/notification）
    pub category: String,

    /// 描述
    pub description: Option<String>,

    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
