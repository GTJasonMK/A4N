//! Add per-provider manual pricing plans used by the model cost ledger.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "llm_config",
            "embedding_config",
            "image_gen_config",
            "video_gen_config",
        ] {
            manager
                .get_connection()
                .execute_unprepared(&format!(
                    "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS pricing_json JSONB NOT NULL DEFAULT '{{}}'::jsonb"
                ))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "video_gen_config",
            "image_gen_config",
            "embedding_config",
            "llm_config",
        ] {
            manager
                .get_connection()
                .execute_unprepared(&format!(
                    "ALTER TABLE {table} DROP COLUMN IF EXISTS pricing_json"
                ))
                .await?;
        }
        Ok(())
    }
}
