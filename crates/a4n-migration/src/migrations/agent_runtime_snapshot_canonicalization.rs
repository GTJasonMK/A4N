//! Canonicalize Agent runtime snapshot admission markers.
//!
//! Every new run stores `runtime_snapshot_required` explicitly. Older rows
//! created before that marker existed are normalized once so runtime recovery
//! no longer needs a missing-field compatibility branch.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Postgres,
                "UPDATE agent_run
                 SET usage_json = jsonb_set(
                     usage_json,
                     '{runtime_snapshot_required}',
                     'false'::jsonb,
                     true
                 )
                 WHERE jsonb_typeof(usage_json) = 'object'
                   AND NOT (usage_json ? 'runtime_snapshot_required')"
                    .to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "agent_runtime_snapshot_canonicalization 不可逆：无法区分历史与新建的 false 标记"
                .to_string(),
        ))
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N Agent runtime snapshot migration only supports PostgreSQL".to_string(),
        ))
    }
}
