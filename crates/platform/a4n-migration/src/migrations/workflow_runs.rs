//! Persistent workflow runs and checkpoints.

mod indexes;
mod rollback;
mod schema;

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        schema::apply(manager).await?;
        indexes::create(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        rollback::apply(manager).await?;
        Ok(())
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        return Ok(());
    }
    Err(DbErr::Custom(
        "A4N workflow run migration only supports PostgreSQL".to_string(),
    ))
}
