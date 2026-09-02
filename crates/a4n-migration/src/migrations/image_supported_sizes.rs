//! Replace the fixed image dimensions with a validated list of supported sizes.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        let db = manager.get_connection();
        execute(
            db,
            "ALTER TABLE image_gen_config ADD COLUMN IF NOT EXISTS supported_sizes_json JSONB",
        )
        .await?;
        execute(
            db,
            r#"DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'image_gen_config'
          AND column_name = 'default_width'
    ) AND EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'image_gen_config'
          AND column_name = 'default_height'
    ) THEN
        EXECUTE 'UPDATE image_gen_config
                 SET supported_sizes_json = jsonb_build_array(
                     jsonb_build_object(''width'', default_width, ''height'', default_height)
                 )
                 WHERE supported_sizes_json IS NULL';
    END IF;
END $$;"#,
        )
        .await?;
        execute(
            db,
            "UPDATE image_gen_config SET supported_sizes_json = '[{\"width\":1024,\"height\":1024}]'::jsonb WHERE supported_sizes_json IS NULL",
        )
        .await?;
        execute(
            db,
            "ALTER TABLE image_gen_config ALTER COLUMN supported_sizes_json SET NOT NULL",
        )
        .await?;
        execute(
            db,
            "ALTER TABLE image_gen_config DROP COLUMN IF EXISTS default_width",
        )
        .await?;
        execute(
            db,
            "ALTER TABLE image_gen_config DROP COLUMN IF EXISTS default_height",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        let db = manager.get_connection();
        execute(
            db,
            "ALTER TABLE image_gen_config ADD COLUMN IF NOT EXISTS default_width INTEGER",
        )
        .await?;
        execute(
            db,
            "ALTER TABLE image_gen_config ADD COLUMN IF NOT EXISTS default_height INTEGER",
        )
        .await?;
        execute(
            db,
            "UPDATE image_gen_config SET default_width = COALESCE((supported_sizes_json -> 0 ->> 'width')::integer, 1024), default_height = COALESCE((supported_sizes_json -> 0 ->> 'height')::integer, 1024)",
        )
        .await?;
        execute(
            db,
            "ALTER TABLE image_gen_config ALTER COLUMN default_width SET NOT NULL",
        )
        .await?;
        execute(
            db,
            "ALTER TABLE image_gen_config ALTER COLUMN default_height SET NOT NULL",
        )
        .await?;
        execute(
            db,
            "ALTER TABLE image_gen_config DROP COLUMN IF EXISTS supported_sizes_json",
        )
        .await?;
        Ok(())
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N image supported sizes migration only supports PostgreSQL".to_string(),
        ))
    }
}

async fn execute<C>(db: &C, sql: &str) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    db.execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
