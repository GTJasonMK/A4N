//! Enforce one persisted default per provider configuration table.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "provider_default_config_integrity_contract";

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
            "{MIGRATION_NAME} 不可逆：默认 Provider 配置只支持收敛后的唯一合同"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.llm_config') IS NULL
       OR to_regclass(current_schema() || '.embedding_config') IS NULL
       OR to_regclass(current_schema() || '.image_gen_config') IS NULL
       OR to_regclass(current_schema() || '.video_gen_config') IS NULL THEN
        RAISE EXCEPTION 'provider_default_config_integrity_contract: provider config table is missing';
    END IF;

    IF (SELECT count(*) FROM llm_config WHERE is_default) > 1
       OR (SELECT count(*) FROM embedding_config WHERE is_default) > 1
       OR (SELECT count(*) FROM image_gen_config WHERE is_default) > 1
       OR (SELECT count(*) FROM video_gen_config WHERE is_default) > 1 THEN
        RAISE EXCEPTION 'provider_default_config_integrity_contract: duplicate provider defaults';
    END IF;
END $$;

CREATE UNIQUE INDEX uidx_llm_config_default
    ON llm_config ((is_default))
    WHERE is_default;

CREATE UNIQUE INDEX uidx_embedding_config_default
    ON embedding_config ((is_default))
    WHERE is_default;

CREATE UNIQUE INDEX uidx_image_gen_config_default
    ON image_gen_config ((is_default))
    WHERE is_default;

CREATE UNIQUE INDEX uidx_video_gen_config_default
    ON video_gen_config ((is_default))
    WHERE is_default;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_rejects_existing_duplicates_and_enforces_each_default() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
            DROP TABLE llm_config, embedding_config, image_gen_config, video_gen_config CASCADE;
            CREATE TABLE llm_config (
                id BIGSERIAL PRIMARY KEY,
                is_default BOOLEAN NOT NULL DEFAULT FALSE,
                updated_at TIMESTAMP NOT NULL DEFAULT now()
            );
            CREATE TABLE embedding_config (LIKE llm_config INCLUDING ALL);
            CREATE TABLE image_gen_config (LIKE llm_config INCLUDING ALL);
            CREATE TABLE video_gen_config (LIKE llm_config INCLUDING ALL);
            INSERT INTO llm_config (is_default) VALUES (TRUE), (TRUE);
            "#,
        )
        .await
        .unwrap();

        assert!(Migration.up(&SchemaManager::new(&db)).await.is_err());
        db.execute_unprepared("DELETE FROM llm_config WHERE id = 2")
            .await
            .unwrap();
        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        for table in [
            "llm_config",
            "embedding_config",
            "image_gen_config",
            "video_gen_config",
        ] {
            if table != "llm_config" {
                db.execute_unprepared(&format!("INSERT INTO {table} (is_default) VALUES (TRUE)"))
                    .await
                    .unwrap();
            }
            assert!(
                db.execute_unprepared(&format!("INSERT INTO {table} (is_default) VALUES (TRUE)"))
                    .await
                    .is_err(),
                "{table} must reject a second default"
            );
        }
    }
}
