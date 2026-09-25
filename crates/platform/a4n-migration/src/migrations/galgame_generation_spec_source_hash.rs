//! Restore the snapshot identity without invalidating already generated assets.

use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

const REPAIR_SQL: &str = r#"
UPDATE generation_spec AS spec
SET source_version_hash = snapshot.source_version_hash
FROM project_source_snapshot AS snapshot
WHERE spec.source_snapshot_id = snapshot.id
  AND spec.root_project_id = snapshot.root_project_id
  AND spec.source_context_kind = 'snapshot_backed'
  AND spec.target_kind IN ('galgame_background', 'galgame_cg', 'portrait_expression')
  AND spec.source_version_hash IS DISTINCT FROM snapshot.source_version_hash;
"#;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Galgame source hash migration requires PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(REPAIR_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "Galgame source hash repair cannot restore incorrect source identities".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Statement, TransactionTrait};

    #[tokio::test]
    async fn repair_changes_only_galgame_snapshot_hashes_and_keeps_asset_status() {
        let db = crate::test_support::connect_test_database().await;
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
            CREATE TEMP TABLE project_source_snapshot (id bigint, root_project_id bigint, source_version_hash text) ON COMMIT DROP;
            CREATE TEMP TABLE generation_spec (id bigint, source_snapshot_id bigint, root_project_id bigint, source_context_kind text, target_kind text, source_version_hash text, status text) ON COMMIT DROP;
            INSERT INTO project_source_snapshot VALUES (10, 1, 'snapshot-hash');
            INSERT INTO generation_spec VALUES
                (1, 10, 1, 'snapshot_backed', 'galgame_background', 'params-hash', 'completed'),
                (2, 10, 1, 'snapshot_backed', 'portrait_expression', 'params-hash', 'draft'),
                (3, 10, 1, 'freeform', 'canvas_video', 'freeform-hash', 'completed'),
                (4, 10, 2, 'snapshot_backed', 'galgame_cg', 'wrong-root', 'completed');
        "#).await.unwrap();
        tx.execute_unprepared(REPAIR_SQL).await.unwrap();
        let rows = tx
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                "SELECT source_version_hash, status FROM generation_spec ORDER BY id".to_string(),
            ))
            .await
            .unwrap();
        let values = rows
            .into_iter()
            .map(|row| {
                (
                    row.try_get::<String>("", "source_version_hash").unwrap(),
                    row.try_get::<String>("", "status").unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            vec![
                ("snapshot-hash".into(), "completed".into()),
                ("snapshot-hash".into(), "draft".into()),
                ("freeform-hash".into(), "completed".into()),
                ("wrong-root".into(), "completed".into()),
            ]
        );
        tx.rollback().await.unwrap();
    }
}
