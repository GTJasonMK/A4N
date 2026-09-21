//! Enforce snapshot context consistency for media provenance references.

mod contract;
mod legacy;
mod triggers;

pub(crate) use self::contract::MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL;
use self::legacy::{
    MEDIA_PROVENANCE_FAMILY_LEGACY_FUNCTION_SQL, MEDIA_PROVENANCE_FAMILY_LEGACY_UPDATE_TRIGGER_SQL,
};
use self::triggers::{
    MEDIA_PROVENANCE_FAMILY_DROP_UPDATE_TRIGGER_SQL, MEDIA_PROVENANCE_FAMILY_UPDATE_TRIGGER_SQL,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media provenance snapshot context migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in [
            MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL,
            MEDIA_PROVENANCE_FAMILY_DROP_UPDATE_TRIGGER_SQL,
            MEDIA_PROVENANCE_FAMILY_UPDATE_TRIGGER_SQL,
        ] {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N media provenance snapshot context migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in [
            MEDIA_PROVENANCE_FAMILY_LEGACY_FUNCTION_SQL,
            MEDIA_PROVENANCE_FAMILY_DROP_UPDATE_TRIGGER_SQL,
            MEDIA_PROVENANCE_FAMILY_LEGACY_UPDATE_TRIGGER_SQL,
        ] {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_provenance_trigger_enforces_galgame_request_snapshot_contract() {
        for expected in [
            "request_snapshot_json 必须是 JSON 对象",
            "request_snapshot_json.source 必须是 galgame_asset_generation",
            "request_snapshot_json.target_entity_kind 缺失",
            "request_snapshot_json.target_entity_id 缺失",
            "request_snapshot_json 与 generation_spec 目标不一致",
            "request_snapshot_json.target_line_id 缺失",
            "request_snapshot_json.target_line_id 必须是整数",
            "request_snapshot_json.target_line_id 必须指向同项目 Galgame 行",
            "source.status = 'completed'",
            "source.file_path IS NOT NULL",
        ] {
            assert!(
                MEDIA_PROVENANCE_FAMILY_FUNCTION_SQL.contains(expected),
                "missing media provenance trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn media_provenance_update_trigger_covers_request_snapshot_json() {
        for expected in [
            "CREATE TRIGGER trg_media_provenance_family_update",
            "BEFORE UPDATE OF media_asset_id, generation_spec_id, prompt_artifact_id, source_media_asset_id, request_snapshot_json ON media_provenance",
        ] {
            assert!(
                MEDIA_PROVENANCE_FAMILY_UPDATE_TRIGGER_SQL.contains(expected),
                "missing media provenance update trigger contract: {expected}"
            );
        }
        assert_eq!(
            MEDIA_PROVENANCE_FAMILY_DROP_UPDATE_TRIGGER_SQL,
            "DROP TRIGGER IF EXISTS trg_media_provenance_family_update ON media_provenance"
        );
    }

    #[test]
    fn legacy_media_provenance_trigger_sql_has_single_body_declaration() {
        assert_eq!(
            MEDIA_PROVENANCE_FAMILY_LEGACY_FUNCTION_SQL
                .matches("AS $$")
                .count(),
            1
        );
    }
}
