//! Enforce the canonical persisted foreshadowing record contract.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "foreshadowing_index_integrity_contract";

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
            "{MIGRATION_NAME} 不可逆：伏笔记录只支持收敛后的唯一合同"
        )))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.foreshadowing_index') IS NULL THEN
        RAISE EXCEPTION
            'foreshadowing_index_integrity_contract: foreshadowing_index table is missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM foreshadowing_index
        WHERE project_id <= 0
           OR description = ''
           OR description <> btrim(description)
           OR setup_chapter <= 0
           OR payoff_chapter <= setup_chapter
           OR status NOT IN ('pending', 'partially_revealed', 'resolved', 'abandoned')
           OR ((status = 'resolved') IS DISTINCT FROM (payoff_chapter IS NOT NULL))
           OR importance NOT BETWEEN 1 AND 5
           OR (related_characters IS NOT NULL AND json_typeof(related_characters) <> 'array')
           OR (
                related_characters IS NOT NULL
                AND jsonb_path_exists(
                    related_characters::jsonb,
                    '$[*] ? (@.type() != "string" || @ like_regex "^\\s*$")'
                )
           )
    ) THEN
        RAISE EXCEPTION
            'foreshadowing_index_integrity_contract: invalid foreshadowing row';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM foreshadowing_index
        GROUP BY project_id, setup_chapter, description
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION
            'foreshadowing_index_integrity_contract: duplicate foreshadowing record';
    END IF;
END $$;

ALTER TABLE foreshadowing_index
    ADD CONSTRAINT ck_foreshadowing_project_id
        CHECK (project_id > 0),
    ADD CONSTRAINT ck_foreshadowing_description
        CHECK (description <> '' AND description = btrim(description)),
    ADD CONSTRAINT ck_foreshadowing_chapters
        CHECK (setup_chapter > 0 AND (payoff_chapter IS NULL OR payoff_chapter > setup_chapter)),
    ADD CONSTRAINT ck_foreshadowing_status
        CHECK (status IN ('pending', 'partially_revealed', 'resolved', 'abandoned')),
    ADD CONSTRAINT ck_foreshadowing_resolution
        CHECK ((status = 'resolved') = (payoff_chapter IS NOT NULL)),
    ADD CONSTRAINT ck_foreshadowing_importance
        CHECK (importance BETWEEN 1 AND 5),
    ADD CONSTRAINT ck_foreshadowing_related_characters
        CHECK (
            related_characters IS NULL
            OR (
                json_typeof(related_characters) = 'array'
                AND NOT jsonb_path_exists(
                    related_characters::jsonb,
                    '$[*] ? (@.type() != "string" || @ like_regex "^\\s*$")'
                )
            )
        );

CREATE UNIQUE INDEX uidx_foreshadowing_project_setup_description
    ON foreshadowing_index (project_id, setup_chapter, description);
"#;

#[cfg(test)]
mod tests {
    use super::UP_SQL;
    use sea_orm::ConnectionTrait;

    #[tokio::test]
    async fn migration_enforces_the_typed_foreshadowing_contract() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
DROP TABLE IF EXISTS foreshadowing_index CASCADE;
CREATE TABLE foreshadowing_index (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL,
    description TEXT NOT NULL,
    setup_chapter INTEGER NOT NULL,
    payoff_chapter INTEGER,
    status TEXT NOT NULL,
    related_characters JSON,
    importance INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
INSERT INTO foreshadowing_index (
    project_id, description, setup_chapter, payoff_chapter,
    status, related_characters, importance
) VALUES (7, '门后的脚步声', 2, NULL, 'pending', '["主角"]', 4);
"#,
        )
        .await
        .expect("canonical foreshadowing fixture must be created");

        db.execute_unprepared(UP_SQL)
            .await
            .expect("foreshadowing integrity migration must succeed");

        for invalid_insert in [
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, importance) VALUES (7, '门后的脚步声', 2, 'pending', 4)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, importance) VALUES (7, '旧信', 3, 'legacy_pending', 4)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, importance) VALUES (7, ' 旧信 ', 3, 'pending', 4)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, payoff_chapter, status, importance) VALUES (7, '旧信', 3, NULL, 'resolved', 4)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, importance) VALUES (7, '旧信', 3, 'pending', 6)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, related_characters, importance) VALUES (7, '旧信', 3, 'pending', '{}', 4)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, related_characters, importance) VALUES (7, '旧信', 3, 'pending', '[9]', 4)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, related_characters, importance) VALUES (7, '旧信', 3, 'pending', '[{\"name\": \"沈砚\"}]', 4)",
            "INSERT INTO foreshadowing_index (project_id, description, setup_chapter, status, related_characters, importance) VALUES (7, '旧信', 3, 'pending', '[\" \"]', 4)",
        ] {
            assert!(db.execute_unprepared(invalid_insert).await.is_err());
        }
    }
}
