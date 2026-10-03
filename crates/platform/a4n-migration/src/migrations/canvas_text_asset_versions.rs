//! Store immutable revisions for project-scoped Canvas text assets.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const UP_SQL: &[&str] = &[
    "ALTER TABLE canvas_text_asset ADD COLUMN IF NOT EXISTS current_version INTEGER NOT NULL DEFAULT 1",
    r#"
CREATE TABLE IF NOT EXISTS canvas_text_asset_version (
    id BIGSERIAL PRIMARY KEY,
    canvas_text_asset_id BIGINT NOT NULL REFERENCES canvas_text_asset(id) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK (version > 0),
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    CONSTRAINT canvas_text_asset_version_unique UNIQUE (canvas_text_asset_id, version)
)
"#,
    r#"
INSERT INTO canvas_text_asset_version
    (canvas_text_asset_id, version, content, content_hash, created_at)
SELECT id, current_version, content, content_hash, created_at
FROM canvas_text_asset
ON CONFLICT (canvas_text_asset_id, version) DO NOTHING
"#,
    r#"
CREATE OR REPLACE FUNCTION reject_canvas_text_asset_version_update()
RETURNS trigger
LANGUAGE plpgsql
AS $$ BEGIN
    RAISE EXCEPTION 'canvas_text_asset_version rows are immutable';
END $$
"#,
    r#"
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'trg_canvas_text_asset_version_immutable'
    ) THEN
        CREATE TRIGGER trg_canvas_text_asset_version_immutable
        BEFORE UPDATE ON canvas_text_asset_version
        FOR EACH ROW EXECUTE FUNCTION reject_canvas_text_asset_version_update();
    END IF;
END $$
"#,
];

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "canvas_text_asset_versions only supports PostgreSQL".to_string(),
            ));
        }
        for sql in UP_SQL {
            manager
                .get_connection()
                .execute(Statement::from_string(DbBackend::Postgres, (*sql).to_string()))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "canvas_text_asset_versions is irreversible because later asset revisions are immutable history".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_backfills_the_existing_asset_as_its_initial_version() {
        assert!(UP_SQL[0].contains("current_version"));
        assert!(UP_SQL[2].contains("SELECT id, current_version, content, content_hash"));
        assert!(UP_SQL[2].contains("ON CONFLICT (canvas_text_asset_id, version) DO NOTHING"));
        assert!(UP_SQL[4].contains("BEFORE UPDATE ON canvas_text_asset_version"));
    }
}
