//! 使用统计实体【新增】

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "usage_stats")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,

    /// 统计日期
    pub date: chrono::NaiveDate,

    /// LLM 调用次数
    pub llm_calls: i32,

    /// Token 总用量
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,

    /// 生成图片数
    pub images_generated: i32,

    /// 生成章节数
    pub chapters_generated: i32,

    /// 活跃项目数
    pub active_projects: i32,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
