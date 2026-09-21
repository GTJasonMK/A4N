//! 使旧视频执行合同失效，同时完整保留已经生成的媒体资产。

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        manager
            .get_connection()
            .execute_unprepared(RESET_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "video_execution_contract_v5_reset 不可逆：旧提示词和检查点已明确失效，媒体资产仅保留归档元数据"
                .to_string(),
        ))
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(
            "A4N video_execution_contract_v5_reset migration only supports PostgreSQL".to_string(),
        ));
    }
    Ok(())
}

const RESET_SQL: &str = r#"
CREATE TEMP TABLE _a4n_v5_invalid_video_prompt ON COMMIT DROP AS
SELECT id, project_id, chapter_id, workflow_mode, prompt_type, shot_number
FROM chapter_video_prompt
WHERE workflow_mode IN ('boundary_frames', 'first_frame_chain', 'storyboard', 'chat_video')
  AND prompt_type IN (
      'frame_segment_plan',
      'frame_prompt',
      'first_frame_prompt',
      'chat_video_frame_prompt',
      'storyboard_prompt',
      'video_prompt'
  );

CREATE INDEX ON _a4n_v5_invalid_video_prompt (id);
CREATE INDEX ON _a4n_v5_invalid_video_prompt (chapter_id, workflow_mode);

UPDATE generated_image image
SET metadata_json = jsonb_set(
    COALESCE(image.metadata_json::jsonb, '{}'::jsonb),
    '{video_chain_archive}',
    jsonb_build_object(
        'contract_epoch', 4,
        'prompt_id', prompt.id,
        'prompt_type', prompt.prompt_type,
        'workflow_mode', prompt.workflow_mode,
        'shot_number', prompt.shot_number,
        'invalidated_reason', 'video_execution_contract_v5'
    ),
    true
)::json
FROM _a4n_v5_invalid_video_prompt prompt
WHERE image.metadata_json->'generation_context'->>'prompt_id' = prompt.id::text
   OR image.metadata_json->>'prompt_id' = prompt.id::text
   OR image.metadata_json->>'chapter_video_prompt_id' = prompt.id::text;

UPDATE generated_video video
SET metadata_json = jsonb_set(
    COALESCE(video.metadata_json::jsonb, '{}'::jsonb),
    '{video_chain_archive}',
    jsonb_build_object(
        'contract_epoch', 4,
        'prompt_id', prompt.id,
        'prompt_type', prompt.prompt_type,
        'workflow_mode', prompt.workflow_mode,
        'shot_number', prompt.shot_number,
        'invalidated_reason', 'video_execution_contract_v5'
    ),
    true
)::json
FROM _a4n_v5_invalid_video_prompt prompt
WHERE video.metadata_json->>'prompt_id' = prompt.id::text
   OR video.metadata_json->'metadata'->>'prompt_id' = prompt.id::text
   OR video.metadata_json->'generation_context'->>'prompt_id' = prompt.id::text
   OR video.metadata_json->>'chapter_video_prompt_id' = prompt.id::text;

CREATE TEMP TABLE _a4n_v5_invalid_workflow_run ON COMMIT DROP AS
SELECT DISTINCT run.id
FROM workflow_run run
WHERE run.workflow_kind = 'chapter_video_prompt_generation'
  AND run.mode IN ('boundary_frames', 'first_frame_chain', 'storyboard', 'chat_video')
  AND EXISTS (
      SELECT 1
      FROM _a4n_v5_invalid_video_prompt prompt
      WHERE prompt.chapter_id = run.chapter_id
        AND prompt.workflow_mode = run.mode
  );

CREATE TEMP TABLE _a4n_v5_invalid_generation_run ON COMMIT DROP AS
SELECT DISTINCT target.generation_run_id AS id
FROM generation_run_target target
JOIN _a4n_v5_invalid_video_prompt prompt
  ON target.target_id = prompt.id
WHERE target.target_kind = 'chapter_video_prompt'
   OR target.target_table = 'chapter_video_prompt'
UNION
SELECT run.id
FROM generation_run run
JOIN _a4n_v5_invalid_workflow_run workflow ON workflow.id = run.workflow_run_id;

DELETE FROM llm_call_trace
WHERE generation_run_id IN (SELECT id FROM _a4n_v5_invalid_generation_run)
   OR workflow_run_id IN (SELECT id FROM _a4n_v5_invalid_workflow_run);

DELETE FROM generation_run_target
WHERE generation_run_id IN (SELECT id FROM _a4n_v5_invalid_generation_run)
   OR (
       (target_kind = 'chapter_video_prompt' OR target_table = 'chapter_video_prompt')
       AND target_id IN (SELECT id FROM _a4n_v5_invalid_video_prompt)
   );

DELETE FROM generation_run
WHERE id IN (SELECT id FROM _a4n_v5_invalid_generation_run);

DELETE FROM workflow_progress_event
WHERE workflow_run_id IN (SELECT id FROM _a4n_v5_invalid_workflow_run);

DELETE FROM workflow_checkpoint
WHERE workflow_run_id IN (SELECT id FROM _a4n_v5_invalid_workflow_run);

DELETE FROM workflow_run
WHERE id IN (SELECT id FROM _a4n_v5_invalid_workflow_run);

DELETE FROM chapter_video_prompt
WHERE id IN (SELECT id FROM _a4n_v5_invalid_video_prompt);
"#;
