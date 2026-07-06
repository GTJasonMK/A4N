//! Provider-specific options for image and video generation configs.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media provider options migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "ALTER TABLE image_gen_config ADD COLUMN IF NOT EXISTS provider_options_json JSONB",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE video_gen_config ADD COLUMN IF NOT EXISTS provider_options_json JSONB",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media provider options migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "ALTER TABLE video_gen_config DROP COLUMN IF EXISTS provider_options_json",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE image_gen_config DROP COLUMN IF EXISTS provider_options_json",
        )
        .await?;
        Ok(())
    }
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
