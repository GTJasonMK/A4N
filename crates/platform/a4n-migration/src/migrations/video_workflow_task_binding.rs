//! 将升级前缺少持久任务绑定的视频运行转换为可恢复中断状态。

use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N video_workflow_task_binding migration only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
UPDATE workflow_run AS workflow
SET status = 'interrupted',
    active_task_id = NULL,
    can_resume = TRUE,
    resume_block_reason = NULL,
    error_message = COALESCE(
        workflow.error_message,
        '升级前视频工作流缺少持久任务绑定，已转换为可恢复中断状态'
    ),
    updated_at = NOW(),
    completed_at = NOW()
WHERE workflow.status = 'running'
  AND workflow.workflow_kind IN ('video_segment_plan', 'video_frame_prompts', 'video_prompts')
  AND NOT EXISTS (
      SELECT 1
      FROM task_queue AS task
      WHERE task.id = workflow.active_task_id
        AND task.workflow_run_id = workflow.id
        AND task.status IN ('pending', 'running')
  );
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "video_workflow_task_binding 不可逆：无法安全恢复升级前的僵尸运行状态".to_string(),
        ))
    }
}
