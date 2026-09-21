//! 项目状态历史实体。

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "project_status_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    /// 旧状态
    pub from_status: String,

    /// 新状态
    pub to_status: String,

    /// 变更原因
    pub reason: Option<String>,

    /// 操作者
    pub changed_by: Option<String>,

    pub changed_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::novel_project::Entity",
        from = "Column::ProjectId",
        to = "super::novel_project::Column::Id"
    )]
    NovelProject,
}

impl ActiveModelBehavior for ActiveModel {}

impl Related<super::novel_project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NovelProject.def()
    }
}
