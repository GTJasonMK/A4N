//! Allow the active Galgame source-sync regeneration strategy in deployed databases.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "source_sync_regenerate_scene_strategy";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(format!(
                "A4N {MIGRATION_NAME} migration only supports PostgreSQL"
            )));
        }
        manager
            .get_connection()
            .execute_unprepared(SOURCE_SYNC_DECISION_FAMILY_FUNCTION_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "source_sync_regenerate_scene_strategy 不可逆：活跃决策策略不会重新禁用".to_string(),
        ))
    }
}

pub(crate) const SOURCE_SYNC_DECISION_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_source_sync_decision_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.decided_by NOT IN ('user','system') THEN
        RAISE EXCEPTION '源同步决策 decided_by 无效';
    END IF;
    IF NEW.strategy NOT IN ('keep_local','adopt_source','rebuild_prompt','rebind_anchor','regenerate_scene','create_replacement','manual','detach','delete_local') THEN
        RAISE EXCEPTION '源同步决策 strategy 无效';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM source_sync_conflict sync_conflict
        WHERE sync_conflict.id = NEW.conflict_id
          AND sync_conflict.status IN ('resolved','ignored')
          AND sync_conflict.resolution_strategy = NEW.strategy
    ) THEN
        RAISE EXCEPTION '源同步决策必须指向已按同一策略处理的冲突';
    END IF;
    RETURN NEW;
END;
$$
"#;

#[cfg(test)]
mod tests {
    use super::SOURCE_SYNC_DECISION_FAMILY_FUNCTION_SQL;
    use a4n_domain::source_sync::SourceSyncResolutionStrategy;
    use sea_orm::{ConnectionTrait, Statement};

    #[test]
    fn migration_allows_all_active_resolution_strategies() {
        for strategy in SourceSyncResolutionStrategy::VALUES {
            assert!(
                SOURCE_SYNC_DECISION_FAMILY_FUNCTION_SQL.contains(&format!("'{strategy}'")),
                "{strategy}"
            );
        }
    }

    #[tokio::test]
    async fn migrated_database_installs_regenerate_scene_strategy() {
        let db = crate::test_support::connect_test_database().await;
        let definition = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT pg_get_functiondef('a4n_validate_source_sync_decision_family()'::regprocedure) AS definition".to_string(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "definition")
            .unwrap();

        assert!(definition.contains("regenerate_scene"));
    }
}
