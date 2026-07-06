//! Split object profile story function into usage-specific fields.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N object profile usage fields migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ADD COLUMN IF NOT EXISTS story_usage TEXT",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ADD COLUMN IF NOT EXISTS visual_usage TEXT",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ADD COLUMN IF NOT EXISTS generation_usage TEXT",
        )
        .await?;
        execute_pg_sql(
            manager,
            r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'object_profile'
          AND column_name = 'function_in_story'
    ) THEN
        EXECUTE $SQL$
        UPDATE object_profile
        SET story_usage = COALESCE(NULLIF(story_usage, ''), function_in_story),
            visual_usage = COALESCE(NULLIF(visual_usage, ''), visual_description),
            generation_usage = COALESCE(NULLIF(generation_usage, ''), image_prompt_seed)
        $SQL$;
    ELSE
        UPDATE object_profile
        SET story_usage = COALESCE(NULLIF(story_usage, ''), visual_description),
            visual_usage = COALESCE(NULLIF(visual_usage, ''), visual_description),
            generation_usage = COALESCE(NULLIF(generation_usage, ''), image_prompt_seed);
    END IF;
END $$
"#,
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ALTER COLUMN story_usage SET NOT NULL",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ALTER COLUMN visual_usage SET NOT NULL",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ALTER COLUMN generation_usage SET NOT NULL",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile DROP COLUMN IF EXISTS function_in_story",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N object profile usage fields migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ADD COLUMN IF NOT EXISTS function_in_story TEXT",
        )
        .await?;
        execute_pg_sql(
            manager,
            r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'object_profile'
          AND column_name = 'story_usage'
    ) THEN
        EXECUTE $SQL$
        UPDATE object_profile
        SET function_in_story = COALESCE(NULLIF(function_in_story, ''), story_usage)
        $SQL$;
    ELSE
        UPDATE object_profile
        SET function_in_story = COALESCE(NULLIF(function_in_story, ''), visual_description);
    END IF;
END $$
"#,
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile ALTER COLUMN function_in_story SET NOT NULL",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile DROP COLUMN IF EXISTS story_usage",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile DROP COLUMN IF EXISTS visual_usage",
        )
        .await?;
        execute_pg_sql(
            manager,
            "ALTER TABLE object_profile DROP COLUMN IF EXISTS generation_usage",
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
