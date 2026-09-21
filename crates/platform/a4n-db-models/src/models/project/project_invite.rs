//! Project collaboration invitations.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "project_invite")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// Root text project id for the shared project family.
    pub project_id: i64,
    pub inviter_user_id: i64,
    pub invitee_user_id: i64,

    /// manager / editor / viewer
    pub role: String,

    /// pending / accepted / declined / revoked / expired
    pub status: String,
    pub expires_at: chrono::NaiveDateTime,
    pub accepted_at: Option<chrono::NaiveDateTime>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
