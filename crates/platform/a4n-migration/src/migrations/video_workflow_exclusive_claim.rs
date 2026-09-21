//! 清理无法归属到任务的旧视频运行状态，为独占 workflow claim 建立明确起点。

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N video_workflow_exclusive_claim migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
UPDATE workflow_run
SET status = 'interrupted',
    can_resume = TRUE,
    resume_block_reason = NULL,
    error_message = COALESCE(error_message, '旧视频工作流缺少 active_task_id，已转换为可恢复中断状态'),
    updated_at = NOW(),
    completed_at = NOW()
WHERE status = 'running'
  AND workflow_kind IN ('video_segment_plan', 'video_frame_prompts', 'video_prompts')
  AND active_task_id IS NULL;
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "video_workflow_exclusive_claim 不可逆：旧 running 状态无法安全恢复".to_string(),
        ))
    }
}
