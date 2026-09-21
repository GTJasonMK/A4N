//! Enforce reverse integrity from Galgame script targets back to generation/media graphs.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

mod function;

pub(crate) use function::GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL;

#[cfg(test)]
mod tests;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Galgame target reverse integrity contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in GALGAME_TARGET_REVERSE_INTEGRITY_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N Galgame target reverse integrity contract migration only supports PostgreSQL"
                    .to_string(),
            ));
        }

        for sql in GALGAME_TARGET_REVERSE_INTEGRITY_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

pub(crate) const GALGAME_TARGET_REVERSE_INTEGRITY_UP_SQL: &[&str] = &[
    GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_script_update ON galgame_script",
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_script_delete ON galgame_script",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_script_delete
AFTER DELETE ON galgame_script
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_scene_update ON galgame_scene",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_scene_update
AFTER UPDATE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_scene_delete ON galgame_scene",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_scene_delete
AFTER DELETE ON galgame_scene
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_line_update ON galgame_line",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_line_update
AFTER UPDATE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_line_delete ON galgame_line",
    r#"
CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_line_delete
AFTER DELETE ON galgame_line
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_target_reverse_integrity()
"#,
];

const GALGAME_TARGET_REVERSE_INTEGRITY_DOWN_SQL: &[&str] = &[
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_line_delete ON galgame_line",
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_line_update ON galgame_line",
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_scene_delete ON galgame_scene",
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_scene_update ON galgame_scene",
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_script_delete ON galgame_script",
    "DROP TRIGGER IF EXISTS trg_galgame_target_reverse_script_update ON galgame_script",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_target_reverse_integrity()",
];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
