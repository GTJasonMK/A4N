//! Removes the duplicate page estimate and retires v2 preprocessing inputs.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                WITH affected AS (
                    SELECT project_id, chapter_id
                    FROM manga_pipeline_artifact
                    WHERE stage = 'preprocessing'
                      AND payload ->> 'schema_version' = 'manga_preprocessing_v2'
                )
                UPDATE manga_pipeline_artifact AS artifact
                SET payload = CASE
                        WHEN artifact.stage = 'preprocessing' THEN jsonb_set(
                            artifact.payload::jsonb #- '{comic_synthesis,total_estimated_pages}',
                            '{schema_version}',
                            '"manga_preprocessing_v3"'::jsonb,
                            TRUE
                        )::json
                        ELSE artifact.payload
                    END,
                    stale = TRUE
                FROM affected
                WHERE artifact.project_id = affected.project_id
                  AND artifact.chapter_id = affected.chapter_id
                "#,
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "manga_preprocessing_contract_v3 不可逆：漫画页数已归属改编计划".into(),
        ))
    }
}
