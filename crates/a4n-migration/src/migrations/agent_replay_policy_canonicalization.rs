//! Canonicalize the replay policy value written by the first Agent runtime
//! prototype.
//!
//! `replayable` was never part of the public Agent contract.  Existing rows
//! may still contain it, so normalize it at the database boundary instead of
//! making every API projection carry a historical alias forever.

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
                "UPDATE agent_execution SET replay_policy = 'safe' WHERE replay_policy = 'replayable'"
                    .to_string(),
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "agent_replay_policy_canonicalization 不可逆：新旧 safe 记录无法可靠区分".to_string(),
        ))
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(DbErr::Custom(
            "A4N Agent replay policy migration only supports PostgreSQL".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn canonicalizes_the_prototype_replay_policy() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared("DELETE FROM agent_execution")
            .await
            .expect("fixture table must be empty");
        db.execute_unprepared(
            "INSERT INTO agent_execution
                (run_id, sequence, execution_key, execution_kind, status, replay_policy, created_at, updated_at)
             VALUES
                (1, 1, 'migration-test-1', 'model_call', 'succeeded', 'replayable', NOW(), NOW()),
                (1, 2, 'migration-test-2', 'model_call', 'succeeded', 'safe', NOW(), NOW())",
        )
        .await
        .expect("fixture rows must be inserted");

        Migration
            .up(&SchemaManager::new(&db))
            .await
            .expect("canonicalization must succeed");

        let rows = db
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                "SELECT replay_policy FROM agent_execution ORDER BY id".to_string(),
            ))
            .await
            .expect("rows must be readable");
        let policies = rows
            .into_iter()
            .map(|row| row.try_get::<String>("", "replay_policy"))
            .collect::<Result<Vec<_>, _>>()
            .expect("policy values must be strings");
        assert_eq!(policies, ["safe", "safe"]);
    }
}
