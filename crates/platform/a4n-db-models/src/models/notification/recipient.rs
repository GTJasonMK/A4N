use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "creator_notification_recipient")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub notification_id: i64,
    pub recipient_user_id: i64,
    pub read_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::announcement::Entity",
        from = "Column::NotificationId",
        to = "super::announcement::Column::Id"
    )]
    Notification,
}

impl Related<super::announcement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Notification.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
