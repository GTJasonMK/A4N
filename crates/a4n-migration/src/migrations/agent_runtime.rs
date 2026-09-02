//! Persistent state for the shared autonomous agent runtime.
//!
//! The runtime deliberately keeps its own lifecycle records. Domain tasks,
//! workflows and generated artifacts remain the source of truth for their
//! respective work; agent rows only coordinate and reference them.

use a4n_core::models::agent::{
    approval, delegation, event, execution, inbox, memory, message, run, thread, turn,
};
use sea_orm::{ConnectionTrait, DbBackend, Schema, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        let schema = Schema::new(DbBackend::Postgres);
        for table in [
            schema.create_table_from_entity(thread::Entity),
            schema.create_table_from_entity(message::Entity),
            schema.create_table_from_entity(turn::Entity),
            schema.create_table_from_entity(run::Entity),
            schema.create_table_from_entity(execution::Entity),
            schema.create_table_from_entity(event::Entity),
            schema.create_table_from_entity(inbox::Entity),
            schema.create_table_from_entity(delegation::Entity),
            schema.create_table_from_entity(approval::Entity),
            schema.create_table_from_entity(memory::Entity),
        ] {
            let mut table = table;
            table.if_not_exists();
            manager.create_table(table).await?;
        }

        // Indexes and constraints are idempotent so a partially applied deploy
        // can be safely resumed.
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_agent_thread_owner_updated ON agent_thread (owner_user_id, updated_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_agent_thread_project ON agent_thread (project_id, updated_at DESC)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_thread_default ON agent_thread (owner_user_id) WHERE is_default = TRUE AND archived_at IS NULL",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_thread_canonical ON agent_thread (owner_user_id, canonical_binding_key) WHERE canonical_binding_key IS NOT NULL AND archived_at IS NULL",
            "CREATE INDEX IF NOT EXISTS idx_agent_message_thread_created ON agent_message (thread_id, created_at, id)",
            // PostgreSQL unique indexes already allow multiple NULL values;
            // keeping this index non-partial lets `ON CONFLICT` infer it.
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_message_client ON agent_message (thread_id, client_message_id)",
            "CREATE INDEX IF NOT EXISTS idx_agent_turn_thread_created ON agent_turn (thread_id, created_at, id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_turn_sequence ON agent_turn (thread_id, sequence)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_turn_client ON agent_turn (thread_id, client_message_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_run_active_thread ON agent_run (thread_id) WHERE is_active = TRUE",
            "CREATE INDEX IF NOT EXISTS idx_agent_run_claim ON agent_run (status, next_attempt_at, lease_expires_at, id)",
            "CREATE INDEX IF NOT EXISTS idx_agent_run_parent ON agent_run (parent_run_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_execution_key ON agent_execution (run_id, execution_key)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_execution_sequence ON agent_execution (run_id, sequence)",
            "CREATE INDEX IF NOT EXISTS idx_agent_execution_status ON agent_execution (run_id, status, sequence)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_event_sequence ON agent_event (thread_id, sequence)",
            "CREATE INDEX IF NOT EXISTS idx_agent_event_thread ON agent_event (thread_id, sequence)",
            "CREATE INDEX IF NOT EXISTS idx_agent_inbox_claim ON agent_inbox (thread_id, status, available_at, priority, id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_inbox_dedupe ON agent_inbox (thread_id, dedupe_key)",
            "CREATE INDEX IF NOT EXISTS idx_agent_delegation_parent ON agent_delegation (parent_run_id, status, id)",
            "CREATE INDEX IF NOT EXISTS idx_agent_delegation_child ON agent_delegation (child_thread_id, child_run_id, status)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_delegation_key ON agent_delegation (parent_run_id, delegation_key)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_approval_execution ON agent_approval (execution_id)",
            "CREATE INDEX IF NOT EXISTS idx_agent_approval_pending ON agent_approval (thread_id, status, expires_at)",
            "CREATE INDEX IF NOT EXISTS idx_agent_memory_scope ON agent_memory (owner_user_id, scope_kind, scope_key, memory_key, revision DESC)",
            "CREATE UNIQUE INDEX IF NOT EXISTS uidx_agent_memory_revision ON agent_memory (owner_user_id, scope_kind, scope_key, memory_key, revision)",
            "ALTER TABLE agent_thread DROP CONSTRAINT IF EXISTS ck_agent_thread_kind",
            "ALTER TABLE agent_thread ADD CONSTRAINT ck_agent_thread_kind CHECK (thread_kind IN ('main', 'subagent'))",
            "ALTER TABLE agent_thread DROP CONSTRAINT IF EXISTS ck_agent_thread_revision",
            "ALTER TABLE agent_thread ADD CONSTRAINT ck_agent_thread_revision CHECK (revision > 0 AND next_turn_sequence >= 0 AND next_event_sequence >= 0)",
            "ALTER TABLE agent_turn DROP CONSTRAINT IF EXISTS ck_agent_turn_sequence",
            "ALTER TABLE agent_turn ADD CONSTRAINT ck_agent_turn_sequence CHECK (sequence > 0)",
            "ALTER TABLE agent_run DROP CONSTRAINT IF EXISTS ck_agent_run_attempt",
            "ALTER TABLE agent_run ADD CONSTRAINT ck_agent_run_attempt CHECK (attempt > 0)",
            "ALTER TABLE agent_run DROP CONSTRAINT IF EXISTS ck_agent_run_active",
            "ALTER TABLE agent_run ADD CONSTRAINT ck_agent_run_active CHECK ((is_active = TRUE AND completed_at IS NULL) OR is_active = FALSE)",
            "ALTER TABLE agent_execution DROP CONSTRAINT IF EXISTS ck_agent_execution_sequence",
            "ALTER TABLE agent_execution ADD CONSTRAINT ck_agent_execution_sequence CHECK (sequence > 0)",
            "ALTER TABLE agent_event DROP CONSTRAINT IF EXISTS ck_agent_event_sequence",
            "ALTER TABLE agent_event ADD CONSTRAINT ck_agent_event_sequence CHECK (sequence > 0)",
            "ALTER TABLE agent_memory DROP CONSTRAINT IF EXISTS ck_agent_memory_revision",
            "ALTER TABLE agent_memory ADD CONSTRAINT ck_agent_memory_revision CHECK (revision > 0)",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        // Drop children first. Foreign keys are intentionally not required by
        // the initial schema, but this order remains safe if they are added.
        for table in [
            "agent_memory",
            "agent_approval",
            "agent_delegation",
            "agent_inbox",
            "agent_event",
            "agent_execution",
            "agent_run",
            "agent_turn",
            "agent_message",
            "agent_thread",
        ] {
            manager
                .drop_table(
                    Table::drop()
                        .table(Alias::new(table))
                        .if_exists()
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N agent runtime migration only supports PostgreSQL".to_string(),
        ))
    }
}
