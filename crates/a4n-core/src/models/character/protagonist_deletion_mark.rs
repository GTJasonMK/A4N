//! 角色删除标记实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "protagonist_deletion_mark")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,

    /// 属性分类：explicit / implicit / social
    pub category: String,

    /// 属性键名
    pub key: String,

    /// 删除候选出现的章节号
    pub chapter_number: i32,

    /// 删除原因
    pub reason: String,

    /// 原文证据
    #[sea_orm(column_type = "Text")]
    pub evidence: String,

    /// 连续命中计数
    pub consecutive_count: i32,

    /// 是否已执行删除
    pub is_executed: bool,

    /// 最后一次命中删除候选的章节号
    pub last_marked_chapter: i32,

    pub created_at: chrono::NaiveDateTime,

    pub updated_at: chrono::NaiveDateTime,
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
