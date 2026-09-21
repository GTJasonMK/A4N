//! Repair stale Agent Run activity flags left by interrupted terminal writes.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Agent Run activity migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "UPDATE agent_run
                 SET is_active = FALSE,
                     worker_id = NULL,
                     lease_token = NULL,
                     lease_expires_at = NULL
                 WHERE is_active = TRUE
                   AND status IN ('completed', 'failed', 'cancelled', 'interrupted', 'recovery_required')"
                    .to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "Agent Run activity canonicalization is not reversible".to_string(),
        ))
    }
}
