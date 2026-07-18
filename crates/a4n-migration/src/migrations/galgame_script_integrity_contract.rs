//! Enforce Galgame script table contracts at the database boundary.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

mod choice;
mod line;
mod scene;

use choice::GALGAME_CHOICE_FAMILY_FUNCTION_SQL;
use line::{GALGAME_LINE_FAMILY_FUNCTION_SQL, GALGAME_LINE_ORDER_FAMILY_FUNCTION_SQL};
use scene::{
    GALGAME_SCENE_FAMILY_FUNCTION_SQL, GALGAME_SCENE_LINK_FAMILY_FUNCTION_SQL,
    GALGAME_SCRIPT_FAMILY_FUNCTION_SQL,
};

#[cfg(test)]
mod tests;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N galgame script integrity contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in GALGAME_SCRIPT_INTEGRITY_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N galgame script integrity contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in GALGAME_SCRIPT_INTEGRITY_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

const GALGAME_SCRIPT_INTEGRITY_UP_SQL: &[&str] = &[
    GALGAME_SCRIPT_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_script_family_insert ON galgame_script",
    r#"
CREATE TRIGGER trg_galgame_script_family_insert
BEFORE INSERT ON galgame_script
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_script_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_script_family_update ON galgame_script",
    r#"
CREATE TRIGGER trg_galgame_script_family_update
BEFORE UPDATE OF project_id, root_project_id, source_snapshot_id, status ON galgame_script
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_script_family()
"#,
    GALGAME_SCENE_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_scene_family_insert ON galgame_scene",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_family_insert
AFTER INSERT ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_scene_family_update ON galgame_scene",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_family_update
AFTER UPDATE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_family()
"#,
    GALGAME_SCENE_LINK_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_scene_link_family_insert ON galgame_scene",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_insert
AFTER INSERT ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_link_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_scene_link_family_update ON galgame_scene",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_update
AFTER UPDATE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_link_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_scene_link_family_delete ON galgame_scene",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_delete
AFTER DELETE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_scene_link_family()
"#,
    GALGAME_LINE_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_line_family_insert ON galgame_line",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_family_insert
AFTER INSERT ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_line_family_update ON galgame_line",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_family_update
AFTER UPDATE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_family()
"#,
    GALGAME_LINE_ORDER_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_line_order_family_insert ON galgame_line",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_insert
AFTER INSERT ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_order_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_line_order_family_update ON galgame_line",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_update
AFTER UPDATE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_order_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_line_order_family_delete ON galgame_line",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_delete
AFTER DELETE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_line_order_family()
"#,
    GALGAME_CHOICE_FAMILY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_choice_family_insert ON galgame_choice",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_insert
AFTER INSERT ON galgame_choice
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_choice_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_choice_family_update ON galgame_choice",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_update
AFTER UPDATE ON galgame_choice
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_choice_family()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_choice_family_delete ON galgame_choice",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_delete
AFTER DELETE ON galgame_choice
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_choice_family()
"#,
];

const GALGAME_SCRIPT_INTEGRITY_DOWN_SQL: &[&str] = &[
    "DROP TRIGGER IF EXISTS trg_galgame_choice_family_delete ON galgame_choice",
    "DROP TRIGGER IF EXISTS trg_galgame_choice_family_update ON galgame_choice",
    "DROP TRIGGER IF EXISTS trg_galgame_choice_family_insert ON galgame_choice",
    "DROP TRIGGER IF EXISTS trg_galgame_line_order_family_delete ON galgame_line",
    "DROP TRIGGER IF EXISTS trg_galgame_line_order_family_update ON galgame_line",
    "DROP TRIGGER IF EXISTS trg_galgame_line_order_family_insert ON galgame_line",
    "DROP TRIGGER IF EXISTS trg_galgame_line_family_update ON galgame_line",
    "DROP TRIGGER IF EXISTS trg_galgame_line_family_insert ON galgame_line",
    "DROP TRIGGER IF EXISTS trg_galgame_scene_link_family_delete ON galgame_scene",
    "DROP TRIGGER IF EXISTS trg_galgame_scene_link_family_update ON galgame_scene",
    "DROP TRIGGER IF EXISTS trg_galgame_scene_link_family_insert ON galgame_scene",
    "DROP TRIGGER IF EXISTS trg_galgame_scene_family_update ON galgame_scene",
    "DROP TRIGGER IF EXISTS trg_galgame_scene_family_insert ON galgame_scene",
    "DROP TRIGGER IF EXISTS trg_galgame_script_family_update ON galgame_script",
    "DROP TRIGGER IF EXISTS trg_galgame_script_family_insert ON galgame_script",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_choice_family()",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_line_order_family()",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_line_family()",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_scene_link_family()",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_scene_family()",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_script_family()",
];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
