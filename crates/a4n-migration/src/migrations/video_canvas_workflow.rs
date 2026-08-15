//! Video canvas workflow schema and project source modes.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N video canvas workflow migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in VIDEO_CANVAS_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N video canvas workflow migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in VIDEO_CANVAS_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

pub(crate) const NOVEL_PROJECT_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_novel_project_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.project_kind NOT IN ('text','comic','video','galgame') THEN
        RAISE EXCEPTION 'novel_project.project_kind 无效';
    END IF;
    IF NEW.source_mode NOT IN ('self','derived','freeform') THEN
        RAISE EXCEPTION 'novel_project.source_mode 无效';
    END IF;
    IF NEW.source_mode = 'self' AND NEW.project_kind <> 'text' THEN
        RAISE EXCEPTION 'self 项目必须是文本项目';
    END IF;
    IF NEW.source_mode = 'derived' AND NEW.project_kind = 'text' THEN
        RAISE EXCEPTION 'derived 项目不能是文本项目';
    END IF;
    IF NEW.source_mode = 'freeform' AND NEW.project_kind <> 'video' THEN
        RAISE EXCEPTION 'freeform 项目当前只支持视频项目';
    END IF;
    IF NEW.source_mode IN ('self','freeform') AND (
        NEW.root_project_id IS NOT NULL
        OR NEW.source_snapshot_id IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'self/freeform 项目不能绑定派生项目字段';
    END IF;
    IF NEW.source_mode = 'derived' AND (
        NEW.root_project_id IS NULL
        OR NEW.source_snapshot_id IS NULL
    ) THEN
        RAISE EXCEPTION '派生项目必须绑定根项目和源快照';
    END IF;
    IF NEW.source_mode = 'derived' AND NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id
          AND root.project_kind = 'text'
          AND root.source_mode = 'self'
    ) THEN
        RAISE EXCEPTION '派生项目 root_project_id 必须指向文本项目';
    END IF;
    IF NEW.source_snapshot_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '派生项目 source_snapshot_id 必须属于 root_project_id';
    END IF;
    RETURN NEW;
END;
$$
"#;

pub(crate) const NOVEL_PROJECT_FAMILY_INSERT_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_novel_project_family_insert
BEFORE INSERT ON novel_project
FOR EACH ROW EXECUTE FUNCTION a4n_validate_novel_project_family()
"#;

pub(crate) const NOVEL_PROJECT_FAMILY_UPDATE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER trg_novel_project_family_update
BEFORE UPDATE OF project_kind, source_mode, root_project_id, source_snapshot_id ON novel_project
FOR EACH ROW EXECUTE FUNCTION a4n_validate_novel_project_family()
"#;

const VIDEO_CANVAS_UP_SQL: &[&str] = &[
    "ALTER TABLE novel_project ADD COLUMN IF NOT EXISTS source_mode TEXT",
    "UPDATE novel_project SET source_mode = CASE WHEN project_kind = 'text' THEN 'self' ELSE 'derived' END WHERE source_mode IS NULL OR source_mode = ''",
    "ALTER TABLE novel_project ALTER COLUMN source_mode SET DEFAULT 'self'",
    "ALTER TABLE novel_project ALTER COLUMN source_mode SET NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_novel_project_source_mode ON novel_project(source_mode)",
    r#"
CREATE TABLE IF NOT EXISTS canvas_graph (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    graph_kind TEXT NOT NULL,
    version INTEGER NOT NULL,
    viewport_json JSONB,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
)
"#,
    r#"
CREATE TABLE IF NOT EXISTS canvas_node (
    id BIGSERIAL PRIMARY KEY,
    graph_id BIGINT NOT NULL REFERENCES canvas_graph(id) ON DELETE CASCADE,
    node_kind TEXT NOT NULL,
    ref_type TEXT,
    ref_id BIGINT,
    config_json JSONB NOT NULL,
    position_json JSONB NOT NULL,
    state TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
)
"#,
    r#"
CREATE TABLE IF NOT EXISTS canvas_edge (
    id BIGSERIAL PRIMARY KEY,
    graph_id BIGINT NOT NULL REFERENCES canvas_graph(id) ON DELETE CASCADE,
    source_node_id BIGINT NOT NULL REFERENCES canvas_node(id) ON DELETE CASCADE,
    source_port TEXT NOT NULL,
    target_node_id BIGINT NOT NULL REFERENCES canvas_node(id) ON DELETE CASCADE,
    target_port TEXT NOT NULL,
    role TEXT NOT NULL,
    config_json JSONB NOT NULL
)
"#,
    r#"
CREATE TABLE IF NOT EXISTS canvas_text_asset (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    text_kind TEXT NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
)
"#,
    "CREATE INDEX IF NOT EXISTS idx_canvas_graph_project_kind ON canvas_graph(project_id, graph_kind)",
    "CREATE INDEX IF NOT EXISTS idx_canvas_node_graph ON canvas_node(graph_id)",
    "CREATE INDEX IF NOT EXISTS idx_canvas_node_ref ON canvas_node(ref_type, ref_id)",
    "CREATE INDEX IF NOT EXISTS idx_canvas_edge_graph ON canvas_edge(graph_id)",
    "CREATE INDEX IF NOT EXISTS idx_canvas_text_asset_project ON canvas_text_asset(project_id)",
    NOVEL_PROJECT_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_novel_project_family_update ON novel_project",
    NOVEL_PROJECT_FAMILY_UPDATE_TRIGGER_SQL,
];

const VIDEO_CANVAS_DOWN_SQL: &[&str] = &[
    "DROP TABLE IF EXISTS canvas_edge",
    "DROP TABLE IF EXISTS canvas_node",
    "DROP TABLE IF EXISTS canvas_text_asset",
    "DROP TABLE IF EXISTS canvas_graph",
    "DROP INDEX IF EXISTS idx_novel_project_source_mode",
    "ALTER TABLE novel_project DROP COLUMN IF EXISTS source_mode",
];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
