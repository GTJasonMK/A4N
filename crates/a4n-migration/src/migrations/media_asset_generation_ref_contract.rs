//! Tighten media asset chapter context and generation media references.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

mod legacy;
mod strict;

use legacy::{
    GENERATION_SPEC_MEDIA_REF_LEGACY_FUNCTION_SQL, MEDIA_ASSET_FAMILY_LEGACY_FUNCTION_SQL,
};
pub(crate) use strict::{
    GENERATION_SPEC_MEDIA_REF_STRICT_FUNCTION_SQL, MEDIA_ASSET_FAMILY_STRICT_FUNCTION_SQL,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media asset generation ref contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in MEDIA_ASSET_GENERATION_REF_CONTRACT_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media asset generation ref contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in MEDIA_ASSET_GENERATION_REF_CONTRACT_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

const MEDIA_ASSET_GENERATION_REF_CONTRACT_UP_SQL: &[&str] = &[
    crate::migrations::galgame_generation_spec_contract::GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
    crate::migrations::media_provenance_snapshot_context::MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL,
    MEDIA_ASSET_FAMILY_STRICT_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_media_asset_family_insert ON media_asset",
    r#"
CREATE TRIGGER trg_media_asset_family_insert
BEFORE INSERT ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
    "DROP TRIGGER IF EXISTS trg_media_asset_family_update ON media_asset",
    r#"
CREATE TRIGGER trg_media_asset_family_update
BEFORE UPDATE OF project_id, root_project_id, source_snapshot_id, source_chapter_id, source_snapshot_chapter_id, status, file_path ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
    GENERATION_SPEC_MEDIA_REF_STRICT_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_generation_spec_media_ref_family_insert ON generation_spec_media_ref",
    r#"
CREATE TRIGGER trg_generation_spec_media_ref_family_insert
BEFORE INSERT ON generation_spec_media_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_media_ref_family()
"#,
    "DROP TRIGGER IF EXISTS trg_generation_spec_media_ref_family_update ON generation_spec_media_ref",
    r#"
CREATE TRIGGER trg_generation_spec_media_ref_family_update
BEFORE UPDATE OF generation_spec_id, media_asset_id, role ON generation_spec_media_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_media_ref_family()
"#,
];

const MEDIA_ASSET_GENERATION_REF_CONTRACT_DOWN_SQL: &[&str] = &[
    MEDIA_ASSET_FAMILY_LEGACY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_media_asset_family_insert ON media_asset",
    r#"
CREATE TRIGGER trg_media_asset_family_insert
BEFORE INSERT ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
    "DROP TRIGGER IF EXISTS trg_media_asset_family_update ON media_asset",
    r#"
CREATE TRIGGER trg_media_asset_family_update
BEFORE UPDATE OF project_id, root_project_id, source_snapshot_id, source_chapter_id, source_snapshot_chapter_id, status, file_path ON media_asset
FOR EACH ROW EXECUTE FUNCTION a4n_validate_media_asset_family()
"#,
    GENERATION_SPEC_MEDIA_REF_LEGACY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_generation_spec_media_ref_family_insert ON generation_spec_media_ref",
    r#"
CREATE TRIGGER trg_generation_spec_media_ref_family_insert
BEFORE INSERT ON generation_spec_media_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_media_ref_family()
"#,
    "DROP TRIGGER IF EXISTS trg_generation_spec_media_ref_family_update ON generation_spec_media_ref",
    r#"
CREATE TRIGGER trg_generation_spec_media_ref_family_update
BEFORE UPDATE OF generation_spec_id, media_asset_id ON generation_spec_media_ref
FOR EACH ROW EXECUTE FUNCTION a4n_validate_generation_spec_media_ref_family()
"#,
];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
