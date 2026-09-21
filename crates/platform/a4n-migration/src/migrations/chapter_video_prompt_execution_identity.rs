//! Make per-segment recipe identity part of the video prompt storage key.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

const EXECUTION_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL: &str = r#"
CREATE UNIQUE INDEX idx_chapter_video_prompt_identity_unique
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
                'storyboard_prompt',
                'video_prompt'
            )
            THEN COALESCE(metadata_json->>'prompt_output_language', 'zh_cn')
            ELSE ''
        END
    ),
    (
        CASE
            WHEN prompt_type IN (
                'frame_prompt',
                'first_frame_prompt',
                'chat_video_frame_prompt',
                'storyboard_prompt',
                'video_prompt'
            )
            THEN COALESCE(metadata_json->'video_execution_identity'->>'segment_plan_prompt_id', '')
            ELSE ''
        END
    ),
    (
        CASE
            WHEN prompt_type IN (
                'frame_prompt',
                'first_frame_prompt',
                'chat_video_frame_prompt',
                'storyboard_prompt',
                'video_prompt'
            )
            THEN COALESCE(metadata_json->'video_execution_identity'->>'segment_number', '')
            ELSE ''
        END
    ),
    (
        CASE
            WHEN prompt_type IN (
                'frame_prompt',
                'first_frame_prompt',
                'chat_video_frame_prompt',
                'storyboard_prompt',
                'video_prompt'
            )
            THEN COALESCE(metadata_json->'video_execution_identity'->>'segment_recipe_fingerprint', '')
            ELSE ''
        END
    )
)
"#;

const LANGUAGE_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL: &str = r#"
CREATE UNIQUE INDEX idx_chapter_video_prompt_identity_unique
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

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        replace_identity_index(manager, EXECUTION_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        replace_identity_index(manager, LANGUAGE_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL).await
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(
            "A4N chapter video prompt execution identity migration only supports PostgreSQL"
                .to_string(),
        ));
    }
    Ok(())
}

async fn replace_identity_index(
    manager: &SchemaManager<'_>,
    create_sql: &str,
) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            DbBackend::Postgres,
            "DROP INDEX IF EXISTS idx_chapter_video_prompt_identity_unique".to_string(),
        ))
        .await?;
    manager
        .get_connection()
        .execute(Statement::from_string(
            DbBackend::Postgres,
            create_sql.to_string(),
        ))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::EXECUTION_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL;

    #[test]
    fn identity_index_separates_segment_recipe_chains() {
        assert!(EXECUTION_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL.contains("storyboard_prompt"));
        assert!(EXECUTION_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL.contains("segment_plan_prompt_id"));
        assert!(EXECUTION_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL.contains("segment_number"));
        assert!(
            EXECUTION_AWARE_VIDEO_PROMPT_IDENTITY_INDEX_SQL.contains("segment_recipe_fingerprint")
        );
    }
}
