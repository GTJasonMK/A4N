//! Remove persisted project source mode after validating the canonical identity fields.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const SOURCE_MODE_VALIDATION_SQL: &str = r#"
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM novel_project
        WHERE NOT (
            (project_kind = 'text'
                AND root_project_id IS NULL
                AND source_snapshot_id IS NULL)
            OR (project_kind = 'video'
                AND root_project_id IS NULL
                AND source_snapshot_id IS NULL)
            OR (project_kind IN ('comic','video','galgame')
                AND root_project_id IS NOT NULL
                AND source_snapshot_id IS NOT NULL)
        )
        OR source_mode IS DISTINCT FROM CASE
            WHEN project_kind = 'text'
                 AND root_project_id IS NULL
                 AND source_snapshot_id IS NULL THEN 'self'
            WHEN project_kind = 'video'
                 AND root_project_id IS NULL
                 AND source_snapshot_id IS NULL THEN 'freeform'
            WHEN project_kind IN ('comic','video','galgame')
                 AND root_project_id IS NOT NULL
                 AND source_snapshot_id IS NOT NULL THEN 'derived'
            ELSE NULL
        END
    ) THEN
        RAISE EXCEPTION 'novel_project.source_mode 与项目身份不一致，拒绝删除重复来源模式';
    END IF;
END;
$$
"#;

const UP_SQL: &[&str] = &[
    SOURCE_MODE_VALIDATION_SQL,
    "DROP TRIGGER IF EXISTS trg_novel_project_family_update ON novel_project",
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_FUNCTION_SQL,
    crate::migrations::galgame_generation_spec_contract::GALGAME_GENERATION_SPEC_FAMILY_FUNCTION_SQL,
    crate::migrations::media_asset_generation_ref_contract::MEDIA_ASSET_FAMILY_STRICT_FUNCTION_SQL,
    crate::migrations::galgame_script_integrity_contract::GALGAME_SCRIPT_FAMILY_FUNCTION_SQL,
    crate::migrations::freeform_canonical_media::VIDEO_SEGMENT_FAMILY_FUNCTION_SQL,
    crate::migrations::freeform_canonical_media::ASSEMBLY_TIMELINE_FAMILY_FUNCTION_SQL,
    "DROP INDEX IF EXISTS idx_novel_project_source_mode",
    "ALTER TABLE novel_project DROP COLUMN source_mode",
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_UPDATE_TRIGGER_SQL,
];

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N project source mode identity cleanup only supports PostgreSQL".to_string(),
            ));
        }
        for sql in UP_SQL {
            manager
                .get_connection()
                .execute(Statement::from_string(
                    DbBackend::Postgres,
                    (*sql).to_string(),
                ))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "project_source_mode_identity_cleanup 不可逆：重复来源模式不会重建".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_validates_then_removes_the_redundant_column() {
        assert_eq!(UP_SQL.first(), Some(&SOURCE_MODE_VALIDATION_SQL));
        assert!(UP_SQL.contains(&"ALTER TABLE novel_project DROP COLUMN source_mode"));
        assert!(
            crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_UPDATE_TRIGGER_SQL
                .contains("project_kind, root_project_id, source_snapshot_id")
        );
    }
}
