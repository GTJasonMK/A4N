//! Require object and scene profile ownership to be explicit on every write.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "profile_record_ownership_default_cleanup";

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
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：档案 ownership_kind 必须由写入方显式提供"
        )))
    }
}

const UP_SQL: &str = r#"
ALTER TABLE object_profile
    ALTER COLUMN ownership_kind DROP DEFAULT;

ALTER TABLE scene_profile
    ALTER COLUMN ownership_kind DROP DEFAULT;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_removes_implicit_profile_ownership() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS object_profile CASCADE;
            DROP TABLE IF EXISTS scene_profile CASCADE;
            CREATE TABLE object_profile (
                id BIGSERIAL PRIMARY KEY,
                ownership_kind TEXT NOT NULL DEFAULT 'legacy_unknown'
            );
            CREATE TABLE scene_profile (
                id BIGSERIAL PRIMARY KEY,
                ownership_kind TEXT NOT NULL DEFAULT 'legacy_unknown'
            );
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        assert!(
            db.execute_unprepared("INSERT INTO object_profile DEFAULT VALUES")
                .await
                .is_err()
        );
        assert!(
            db.execute_unprepared("INSERT INTO scene_profile DEFAULT VALUES")
                .await
                .is_err()
        );
        db.execute_unprepared(
            "INSERT INTO object_profile (ownership_kind) VALUES ('legacy_unknown')",
        )
        .await
        .unwrap();
        db.execute_unprepared("INSERT INTO scene_profile (ownership_kind) VALUES ('user')")
            .await
            .unwrap();
    }
}
