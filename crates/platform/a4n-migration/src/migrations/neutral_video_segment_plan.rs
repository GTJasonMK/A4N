//! 将章节视频片段规划从具体提示词模式中剥离为章节级共享身份。

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N neutral_video_segment_plan migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
UPDATE chapter_video_prompt
SET workflow_mode = 'shared_segment_plan',
    updated_at = NOW()
WHERE workflow_mode = 'boundary_frames'
  AND prompt_type = 'frame_segment_plan'
  AND shot_number = 0;

UPDATE workflow_run
SET business_key = CONCAT(
        'video_prompt:video_segment_plan:',
        project_id,
        ':',
        chapter_id,
        ':shared_segment_plan'
    ),
    mode = 'shared_segment_plan',
    input_fingerprint_json = (
        COALESCE(input_fingerprint_json::jsonb, '{}'::jsonb)
        || jsonb_build_object('workflow_mode', 'shared_segment_plan')
    )::json,
    updated_at = NOW()
WHERE workflow_kind = 'video_segment_plan'
  AND project_id IS NOT NULL
  AND chapter_id IS NOT NULL;
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "neutral_video_segment_plan 不可逆：共享片段规划不能安全恢复为某个具体下游模式"
                .to_string(),
        ))
    }
}
