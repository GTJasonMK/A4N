//! Remove the duplicated project-derivation identity and keep novel_project as the owner.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const REDUNDANT_IDENTITY_VALIDATION_SQL: &str = r#"
DO $$
DECLARE
    has_conflicts BOOLEAN;
BEGIN
    IF to_regclass('project_derivation') IS NOT NULL THEN
        EXECUTE $query$
            SELECT EXISTS (
                SELECT 1
                FROM project_derivation derivation
                LEFT JOIN novel_project derived ON derived.id = derivation.derived_project_id
                LEFT JOIN project_source_snapshot snapshot ON snapshot.id = derivation.source_snapshot_id
                WHERE derived.id IS NULL
                   OR derivation.root_project_id IS DISTINCT FROM derived.root_project_id
                   OR derivation.derived_kind IS DISTINCT FROM derived.project_kind
                   OR derivation.source_snapshot_id IS DISTINCT FROM derived.source_snapshot_id
                   OR snapshot.id IS NULL
                   OR snapshot.root_project_id IS DISTINCT FROM derivation.root_project_id
                   OR derivation.chapter_scope_json::jsonb IS DISTINCT FROM snapshot.chapter_scope_json::jsonb
                   OR derivation.creation_options_json::jsonb IS DISTINCT FROM '{}'::jsonb
            )
        $query$ INTO has_conflicts;
        IF has_conflicts THEN
            RAISE EXCEPTION 'project_derivation 与项目主体或源快照不一致，拒绝删除重复身份';
        END IF;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'novel_project'
          AND column_name = 'parent_project_id'
    ) THEN
        EXECUTE $query$
            SELECT EXISTS (
                SELECT 1 FROM novel_project
                WHERE (source_mode = 'derived' AND parent_project_id IS DISTINCT FROM root_project_id)
                   OR (source_mode <> 'derived' AND parent_project_id IS NOT NULL)
            )
        $query$ INTO has_conflicts;
        IF has_conflicts THEN
            RAISE EXCEPTION 'novel_project.parent_project_id 含有独立业务身份，拒绝删除';
        END IF;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'novel_project'
          AND column_name = 'derived_project_status'
    ) THEN
        EXECUTE $query$
            SELECT EXISTS (
                SELECT 1 FROM novel_project
                WHERE (source_mode = 'derived' AND derived_project_status IS DISTINCT FROM 'draft')
                   OR (source_mode <> 'derived' AND derived_project_status IS NOT NULL)
            )
        $query$ INTO has_conflicts;
        IF has_conflicts THEN
            RAISE EXCEPTION 'novel_project.derived_project_status 含有非固定状态，拒绝删除';
        END IF;
    END IF;
END;
$$
"#;

const UP_SQL: &[&str] = &[
    REDUNDANT_IDENTITY_VALIDATION_SQL,
    "DROP TRIGGER IF EXISTS trg_novel_project_family_insert ON novel_project",
    "DROP TRIGGER IF EXISTS trg_novel_project_family_update ON novel_project",
    "DROP TABLE IF EXISTS project_derivation",
    "DROP FUNCTION IF EXISTS a4n_validate_project_derivation_family()",
    "ALTER TABLE novel_project DROP COLUMN IF EXISTS parent_project_id",
    "ALTER TABLE novel_project DROP COLUMN IF EXISTS derived_project_status",
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_FUNCTION_SQL,
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_INSERT_TRIGGER_SQL,
    crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_UPDATE_TRIGGER_SQL,
];

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N project derivation identity cleanup only supports PostgreSQL".to_string(),
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
            "project_derivation_identity_cleanup 不可逆：重复身份数据不会重建".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_rejects_divergent_data_and_rebuilds_the_single_identity_trigger() {
        assert!(REDUNDANT_IDENTITY_VALIDATION_SQL.contains("拒绝删除重复身份"));
        assert!(UP_SQL.contains(&"DROP TABLE IF EXISTS project_derivation"));
        assert!(
            UP_SQL.contains(&"ALTER TABLE novel_project DROP COLUMN IF EXISTS parent_project_id")
        );
        assert!(
            UP_SQL.contains(
                &"ALTER TABLE novel_project DROP COLUMN IF EXISTS derived_project_status"
            )
        );
        assert!(
            crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_UPDATE_TRIGGER_SQL
                .contains("root_project_id, source_snapshot_id")
        );
    }

    #[tokio::test]
    async fn cleanup_executes_against_the_legacy_table_and_columns() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(
            r#"
ALTER TABLE novel_project ADD COLUMN parent_project_id BIGINT;
ALTER TABLE novel_project ADD COLUMN derived_project_status TEXT;
CREATE TABLE project_derivation (
    id BIGSERIAL PRIMARY KEY,
    root_project_id BIGINT NOT NULL,
    derived_project_id BIGINT NOT NULL,
    derived_kind TEXT NOT NULL,
    source_snapshot_id BIGINT NOT NULL,
    chapter_scope_json JSONB NOT NULL,
    creation_options_json JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL
);
"#,
        )
        .await
        .unwrap();

        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        assert!(
            db.query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT parent_project_id FROM novel_project LIMIT 1".to_string(),
            ))
            .await
            .is_err()
        );
        assert!(
            db.query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT 1 FROM project_derivation LIMIT 1".to_string(),
            ))
            .await
            .is_err()
        );
    }
}
