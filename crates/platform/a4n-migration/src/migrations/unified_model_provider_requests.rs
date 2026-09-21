use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N unified provider migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'llm_config' AND column_name = 'request_options_json'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'llm_config' AND column_name = 'extra_body_json'
    ) THEN
        ALTER TABLE llm_config RENAME COLUMN request_options_json TO extra_body_json;
    ELSE
        ALTER TABLE llm_config ADD COLUMN IF NOT EXISTS extra_body_json JSONB;
        IF EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_name = 'llm_config' AND column_name = 'request_options_json'
        ) THEN
            UPDATE llm_config
            SET extra_body_json = request_options_json
            WHERE extra_body_json IS NULL AND request_options_json IS NOT NULL;
            ALTER TABLE llm_config DROP COLUMN request_options_json;
        END IF;
    END IF;
    -- RENAME 分支会保留源列的 json 类型，统一规整为 jsonb。
    ALTER TABLE llm_config
        ALTER COLUMN extra_body_json TYPE JSONB USING extra_body_json::jsonb;
END $$;
ALTER TABLE image_gen_config DROP COLUMN IF EXISTS provider_options_json;
ALTER TABLE video_gen_config DROP COLUMN IF EXISTS provider_options_json;
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N unified provider migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE llm_config RENAME COLUMN extra_body_json TO request_options_json;
ALTER TABLE image_gen_config ADD COLUMN IF NOT EXISTS provider_options_json JSONB;
ALTER TABLE video_gen_config ADD COLUMN IF NOT EXISTS provider_options_json JSONB;
"#,
            )
            .await?;
        Ok(())
    }
}
