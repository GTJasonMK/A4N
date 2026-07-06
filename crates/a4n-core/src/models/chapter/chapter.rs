//! 章节实体

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub const STATUS_DRAFT: &str = "draft";
pub const STATUS_GENERATING: &str = "generating";
pub const STATUS_IMPORTED: &str = "imported";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_REVIEWED: &str = "reviewed";
pub const STATUS_PUBLISHED: &str = "published";
pub const STATUS_DELETED: &str = "deleted";

pub fn is_valid_status(status: &str) -> bool {
    matches!(
        status,
        STATUS_DRAFT
            | STATUS_GENERATING
            | STATUS_IMPORTED
            | STATUS_COMPLETED
            | STATUS_REVIEWED
            | STATUS_PUBLISHED
            | STATUS_DELETED
    )
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chapter")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub outline_id: Option<i64>,

    pub chapter_number: i32,
    pub title: Option<String>,

    /// 章节正文
    #[sea_orm(column_type = "Text")]
    pub content: String,

    pub word_count: i32,

    /// 状态：draft / generating / imported / completed / reviewed / published / deleted
    pub status: String,

    /// 章节摘要
    #[sea_orm(column_type = "Text")]
    pub summary: Option<String>,

    /// 风格标签
    #[sea_orm(column_type = "Json")]
    pub style_tags: Option<serde_json::Value>,

    /// 章节结构化分析数据
    #[sea_orm(column_type = "Json")]
    pub analysis_data: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Model {
    pub fn has_content(&self) -> bool {
        !self.content.trim().is_empty()
    }

    pub fn has_available_content(&self) -> bool {
        self.status != STATUS_DELETED && self.has_content()
    }

    pub fn is_generating(&self) -> bool {
        self.status == STATUS_GENERATING
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::super::project::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::super::project::novel_project::Column::Id"
    )]
    NovelProject,
    #[sea_orm(has_many = "super::chapter_scene::Entity")]
    ChapterScene,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::super::project::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}

impl Related<super::chapter_scene::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChapterScene.def()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter_with(status: &str, content: &str) -> Model {
        let now = chrono::Utc::now().naive_utc();
        Model {
            id: 1,
            project_id: 1,
            outline_id: None,
            chapter_number: 1,
            title: None,
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: status.to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn has_available_content_rejects_deleted_chapter_content() {
        let deleted = chapter_with(STATUS_DELETED, "已删除正文");

        assert!(deleted.has_content());
        assert!(!deleted.has_available_content());
    }

    #[test]
    fn has_available_content_accepts_non_deleted_content() {
        let imported = chapter_with(STATUS_IMPORTED, "有效正文");
        let empty = chapter_with(STATUS_IMPORTED, "  ");

        assert!(imported.has_available_content());
        assert!(!empty.has_available_content());
    }
}
