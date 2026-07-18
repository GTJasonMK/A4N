//! Historical migration marker retained for databases that already recorded this version.
//!
//! The retired legacy table is no longer part of the application schema. Fresh databases
//! receive `chapter_shooting_script` from the baseline/final hard-cut migration instead.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
