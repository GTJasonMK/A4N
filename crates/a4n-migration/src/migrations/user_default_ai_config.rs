//! Historical default AI config migration marker.
//!
//! Some deployed databases have already recorded `user_default_ai_config` in
//! `seaql_migrations`. Keep this migration registered so SeaORM can verify
//! migration history after the PostgreSQL baseline consolidation.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N user default AI config migration only supports PostgreSQL".to_string(),
            ));
        }

        let db = manager.get_connection();
        for sql in DEFAULT_AI_CONFIG_SQL {
            db.execute(Statement::from_string(
                DbBackend::Postgres,
                (*sql).to_string(),
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N user default AI config migration only supports PostgreSQL".to_string(),
            ));
        }
        Ok(())
    }
}

const DEFAULT_AI_CONFIG_SQL: &[&str] = &[
    "ALTER TABLE llm_config ADD COLUMN IF NOT EXISTS is_default BOOLEAN NOT NULL DEFAULT false",
    "ALTER TABLE image_gen_config ADD COLUMN IF NOT EXISTS is_default BOOLEAN NOT NULL DEFAULT false",
    "ALTER TABLE video_gen_config ADD COLUMN IF NOT EXISTS is_default BOOLEAN NOT NULL DEFAULT false",
];
