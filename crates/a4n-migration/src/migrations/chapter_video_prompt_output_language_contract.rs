//! Enforce canonical output-language metadata on active video prompt records.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "chapter_video_prompt_output_language_contract";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(format!(
                "A4N {MIGRATION_NAME} migration only supports PostgreSQL"
            )));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：视频提示词输出语言只支持收敛后的唯一合同"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.chapter_video_prompt') IS NULL THEN
        RAISE EXCEPTION
            'chapter_video_prompt_output_language_contract: chapter_video_prompt table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM chapter_video_prompt
        WHERE prompt_type IN (
            'frame_prompt',
            'first_frame_prompt',
            'chat_video_frame_prompt',
            'storyboard_prompt',
            'video_prompt'
        )
          AND NOT COALESCE(
              json_typeof(metadata_json) = 'object'
              AND metadata_json->>'prompt_output_language' IN ('zh_cn', 'en'),
              FALSE
          )
    ) THEN
        RAISE EXCEPTION
            'chapter_video_prompt_output_language_contract: active video prompt has missing or invalid prompt_output_language';
    END IF;
END $$;

ALTER TABLE chapter_video_prompt
    ADD CONSTRAINT ck_chapter_video_prompt_output_language
    CHECK (
        prompt_type NOT IN (
            'frame_prompt',
            'first_frame_prompt',
            'chat_video_frame_prompt',
            'storyboard_prompt',
            'video_prompt'
        )
        OR COALESCE(
            json_typeof(metadata_json) = 'object'
            AND metadata_json->>'prompt_output_language' IN ('zh_cn', 'en'),
            FALSE
        )
    );
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_enforces_canonical_video_prompt_output_language() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS chapter_video_prompt CASCADE;
            CREATE TABLE chapter_video_prompt (
                id BIGSERIAL PRIMARY KEY,
                prompt_type TEXT NOT NULL,
                metadata_json JSON
            );
            INSERT INTO chapter_video_prompt (prompt_type, metadata_json) VALUES
                ('frame_prompt', '{"prompt_output_language":"zh_cn"}'),
                ('video_prompt', '{"prompt_output_language":"en"}'),
                ('frame_segment_plan', NULL);
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        for invalid_insert in [
            "INSERT INTO chapter_video_prompt (prompt_type, metadata_json) VALUES ('frame_prompt', NULL)",
            r#"INSERT INTO chapter_video_prompt (prompt_type, metadata_json) VALUES ('video_prompt', '{}')"#,
            r#"INSERT INTO chapter_video_prompt (prompt_type, metadata_json) VALUES ('storyboard_prompt', '{"prompt_output_language":" en "}')"#,
            r#"INSERT INTO chapter_video_prompt (prompt_type, metadata_json) VALUES ('first_frame_prompt', '{"prompt_output_language":"invalid"}')"#,
        ] {
            assert!(db.execute_unprepared(invalid_insert).await.is_err());
        }
    }
}
