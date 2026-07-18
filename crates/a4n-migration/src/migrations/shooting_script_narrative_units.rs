//! Hard cut from whole-chapter shot design JSON to narrative-unit shooting scripts.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N shooting script narrative-unit migration only supports PostgreSQL".into(),
            ));
        }

        // Old JSON cannot satisfy the new required identity/timeline contract. Regeneration is the
        // only truthful migration; synthesizing defaults would create a second, ambiguous source.
        for sql in [
            "DELETE FROM chapter_shooting_script",
            "DELETE FROM workflow_checkpoint WHERE workflow_run_id IN (SELECT id FROM workflow_run WHERE workflow_kind = 'chapter_shooting_script')",
            "UPDATE workflow_run SET status = 'cancelled', can_resume = FALSE, resume_block_reason = 'shooting_script_narrative_unit_contract_changed', active_task_id = NULL, updated_at = NOW() WHERE workflow_kind = 'chapter_shooting_script' AND status <> 'cancelled'",
            "UPDATE novel_project SET source_sync_status = 'root_updated', derived_project_status = 'draft', updated_at = NOW() WHERE project_kind IN ('video', 'comic', 'galgame') AND source_mode = 'derived'",
        ] {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "shooting_script_narrative_units is an irreversible contract hard cut".into(),
        ))
    }
}
