//! Language-aware video prompt identity.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

const LANGUAGE_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS idx_chapter_video_prompt_identity_unique
ON chapter_video_prompt (
    chapter_id,
    workflow_mode,
    prompt_type,
    shot_number,
    (
        CASE
            WHEN prompt_type IN (
                'frame_prompt',
                'first_frame_prompt',
                'chat_video_frame_prompt',
                'video_prompt'
            )
            THEN COALESCE(metadata_json->>'prompt_output_language', 'zh_cn')
            ELSE ''
        END
    )
)
"#;

const LEGACY_VIDEO_PROMPT_IDENTITY_INDEX_SQL: &str = r#"
CREATE UNIQUE INDEX IF NOT EXISTS idx_chapter_video_prompt_identity_unique
ON chapter_video_prompt (
    chapter_id,
    workflow_mode,
    prompt_type,
    shot_number
)
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter video prompt language identity migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "DROP INDEX IF EXISTS idx_chapter_video_prompt_identity_unique",
        )
        .await?;
        execute_pg_sql(manager, LANGUAGE_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N chapter video prompt language identity migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        execute_pg_sql(
            manager,
            "DROP INDEX IF EXISTS idx_chapter_video_prompt_identity_unique",
        )
        .await?;
        execute_pg_sql(manager, LEGACY_VIDEO_PROMPT_IDENTITY_INDEX_SQL).await?;
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
