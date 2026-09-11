//! 章节大纲派生来源记录。
//!
//! 这张表只记录系统自动生成的派生来源。没有记录的章纲按用户拥有处理，
//! 这样正文分析刷新时不会误覆盖用户确认过的资料。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 章节分析完成后生成的回顾性章纲来源。
pub const SOURCE_KIND_RETROSPECTIVE_ANALYSIS: &str = "retrospective_analysis";

/// 导入分析阶段生成的章节章纲来源。
pub const SOURCE_KIND_IMPORT_ANALYSIS: &str = "import_analysis";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter_outline_provenance")]
pub struct Model {
    /// 与 chapter_outline 一对一；使用章纲 ID 作为主键，避免第二份业务身份。
    #[sea_orm(primary_key, auto_increment = false)]
    pub outline_id: i64,
    pub source_kind: String,
    #[sea_orm(column_type = "Json")]
    pub source_fingerprint_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::chapter_outline::Entity",
        from = "Column::OutlineId",
        to = "super::chapter_outline::Column::Id",
        on_delete = "Cascade"
    )]
    ChapterOutline,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::chapter_outline::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChapterOutline.def()
    }
}
