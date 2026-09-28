//! Chapter identities are relative to the target project, including derived videos.
use sea_orm::{ConnectionTrait, DbErr};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "CREATE TABLE visual_plan_edit_receipt (
                id BIGSERIAL PRIMARY KEY,
                project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
                source_chapter_id BIGINT NOT NULL,
                operation_id VARCHAR(128) NOT NULL,
                payload_hash TEXT NOT NULL,
                record_id BIGINT NOT NULL REFERENCES chapter_visual_plan(id) ON DELETE CASCADE,
                before_record JSONB NOT NULL,
                after_record JSONB NOT NULL,
                changes JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE(project_id, operation_id)
            );
            CREATE INDEX idx_visual_plan_edit_receipt_cursor ON visual_plan_edit_receipt(project_id, source_chapter_id, id)"
        ).await?;
        manager.get_connection().execute_unprepared(
            "UPDATE agent_execution e SET status = 'outcome_unknown', error_message = '编辑协议已升级；旧的进行中操作必须人工核对，禁止自动重放'
               FROM agent_run r, agent_thread t
              WHERE e.run_id = r.id AND r.thread_id = t.id AND t.profile_id = 'director'
                AND t.canonical_binding_key LIKE 'subagent:director:chapter:%'
                AND e.status IN ('prepared','awaiting_user','dispatching','running');
             UPDATE agent_run r SET status = 'interrupted', is_active = false, waiting_reason = NULL,
                worker_id = NULL, lease_token = NULL, lease_expires_at = NULL,
                last_error = '编辑协议已升级，请核对已保存结果后发起新请求', updated_at = now(), completed_at = now()
               FROM agent_thread t WHERE r.thread_id = t.id AND t.profile_id = 'director'
                AND t.canonical_binding_key LIKE 'subagent:director:chapter:%' AND r.is_active = true;
             UPDATE agent_turn u SET status = 'failed', error_message = '编辑协议已升级，请发起新请求'
               FROM agent_thread t WHERE u.thread_id = t.id AND t.profile_id = 'director'
                AND t.canonical_binding_key LIKE 'subagent:director:chapter:%' AND u.status IN ('queued','active');
             UPDATE agent_thread t SET status = 'archived', archived_at = COALESCE(t.archived_at, now()), revision = t.revision + 1
               FROM novel_project p WHERE p.id = t.project_id AND t.profile_id = 'director' AND p.project_kind <> 'video'
                AND t.canonical_binding_key LIKE 'subagent:director:chapter:%'"
        ).await?;
        manager.get_connection().execute_unprepared(
            "UPDATE agent_thread t
                SET canonical_binding_key = 'subagent:v2:' || t.profile_id || ':project:' || t.project_id::text || ':chapter:' || t.resource_id
               FROM novel_project p
              WHERE p.id = t.project_id AND t.resource_type = 'chapter'
                AND t.scope_json->>'chapter_id' = t.resource_id
                AND t.scope_json->>'project_id' = t.project_id::text
                AND t.canonical_binding_key = 'subagent:' || t.profile_id || ':chapter:' || t.resource_id
                AND (t.profile_id <> 'director' OR p.project_kind = 'video')"
        ).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "项目限定的 Agent 身份不能回退到有碰撞的章节键".into(),
        ))
    }
}
