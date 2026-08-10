//! 角色行为记录实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "protagonist_behavior_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    /// 记录来源：user / ai_sync / legacy_unknown。
    pub source_kind: String,

    #[sea_orm(column_type = "Text")]
    pub behavior_description: String,

    /// 行为类型
    pub behavior_type: String,

    /// 原文摘录，作为行为依据
    #[sea_orm(column_type = "Text")]
    pub original_text: String,

    /// 行为标签
    #[sea_orm(column_type = "Json")]
    pub behavior_tags: serde_json::Value,

    /// 隐性属性分类结果
    #[sea_orm(column_type = "Json")]
    pub classification_results: serde_json::Value,

    /// 相关角色
    #[sea_orm(column_type = "Json")]
    pub related_characters: Option<serde_json::Value>,

    /// 影响分析
    #[sea_orm(column_type = "Text")]
    pub impact_analysis: Option<String>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::protagonist_profile::Entity",
        from = "Column::ProfileId",
        to = "super::protagonist_profile::Column::Id"
    )]
    ProtagonistProfile,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::protagonist_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProtagonistProfile.def()
    }
}
