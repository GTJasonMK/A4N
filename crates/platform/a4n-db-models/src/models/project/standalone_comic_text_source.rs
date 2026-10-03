//! Frozen project-text source used by standalone comic projects.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "standalone_comic_text_source")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source_snapshot_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub canvas_text_asset_id: i64,
    pub text_asset_version: i32,
    pub start_section_index: i32,
    pub end_section_index: i32,
    pub source_title: String,
    pub content_hash: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::novel_project::Column::Id",
        on_delete = "Cascade"
    )]
    Project,
    #[sea_orm(
        belongs_to = "super::super::chapter::chapter::Entity",
        from = "Column::ChapterId",
        to = "super::super::chapter::chapter::Column::Id",
        on_delete = "Cascade"
    )]
    Chapter,
    #[sea_orm(
        belongs_to = "super::project_source_snapshot::Entity",
        from = "Column::SourceSnapshotId",
        to = "super::project_source_snapshot::Column::Id",
        on_delete = "Cascade"
    )]
    Snapshot,
    #[sea_orm(
        belongs_to = "super::source_snapshot_chapter::Entity",
        from = "Column::SourceSnapshotChapterId",
        to = "super::source_snapshot_chapter::Column::Id",
        on_delete = "Cascade"
    )]
    SnapshotChapter,
    #[sea_orm(
        belongs_to = "super::super::canvas::canvas_text_asset::Entity",
        from = "Column::CanvasTextAssetId",
        to = "super::super::canvas::canvas_text_asset::Column::Id",
        on_delete = "Restrict"
    )]
    TextAsset,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl Related<super::super::chapter::chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chapter.def()
    }
}

impl Related<super::project_source_snapshot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Snapshot.def()
    }
}

impl Related<super::source_snapshot_chapter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SnapshotChapter.def()
    }
}

impl Related<super::super::canvas::canvas_text_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TextAsset.def()
    }
}
