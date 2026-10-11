//! Convert the retired account-level read-only role to the regular user role.

use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N user role removal migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared("UPDATE \"user\" SET role = 'user' WHERE role = 'read_only'")
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "user_read_only_role_removal 不可逆：旧账号已转换为普通用户".to_string(),
        ))
    }
}
