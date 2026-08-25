//! Expand theme background resource files from 8 MiB to 20 MiB.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "theme_resource_size_limit_v2";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(format!(
            "{MIGRATION_NAME} 不可逆：主题资源大小上限已放宽到 20 MiB"
        )))
    }
}

const UP_SQL: &str = r#"
ALTER TABLE theme_resource
    DROP CONSTRAINT IF EXISTS theme_resource_byte_size_check,
    DROP CONSTRAINT IF EXISTS ck_theme_resource_byte_size,
    ADD CONSTRAINT ck_theme_resource_byte_size
        CHECK (byte_size > 0 AND byte_size <= 20971520);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ConnectionTrait;

    #[tokio::test]
    async fn migration_allows_theme_resources_up_to_20_mib() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS theme_resource CASCADE;
            CREATE TABLE theme_resource (
                byte_size BIGINT NOT NULL,
                CONSTRAINT theme_resource_byte_size_check
                    CHECK (byte_size > 0 AND byte_size <= 8388608)
            );
            "#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        db.execute_unprepared("INSERT INTO theme_resource (byte_size) VALUES (20971520)")
            .await
            .unwrap();
        assert!(
            db.execute_unprepared("INSERT INTO theme_resource (byte_size) VALUES (20971521)")
                .await
                .is_err()
        );
    }
}
