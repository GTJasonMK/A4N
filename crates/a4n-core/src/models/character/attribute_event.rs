//! 属性事件实体【新增】—— 事件溯源存储

use a4n_domain::character::CharacterTimelineSourceKind;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 时间线记录来源。旧数据库无法可靠回溯来源，迁移时统一标为未知。
pub const SOURCE_KIND_LEGACY_UNKNOWN: &str = CharacterTimelineSourceKind::LegacyUnknown.as_str();
pub const SOURCE_KIND_USER: &str = CharacterTimelineSourceKind::User.as_str();
pub const SOURCE_KIND_AI_SYNC: &str = CharacterTimelineSourceKind::AiSync.as_str();

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "attribute_event")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub timestamp: chrono::NaiveDateTime,

    /// 属性分类：explicit / implicit / social
    pub category: String,

    /// 属性键名
    pub key: String,

    /// 操作类型：add / modify / delete
    pub operation: String,

    /// 变更摘要
    #[sea_orm(column_type = "Text")]
    pub change_description: String,

    /// 变更前值
    #[sea_orm(column_type = "Json")]
    pub old_value: serde_json::Value,

    /// 变更后值
    #[sea_orm(column_type = "Json")]
    pub new_value: serde_json::Value,

    /// 触发原因
    pub cause: String,

    /// 原文证据
    #[sea_orm(column_type = "Text")]
    pub evidence: String,
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
