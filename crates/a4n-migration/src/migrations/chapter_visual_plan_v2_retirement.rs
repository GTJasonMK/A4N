//! Remove persisted chapter visual plans written with the retired V2 contract.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "chapter_visual_plan_v2_retirement";

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
            "{MIGRATION_NAME} 不可逆：chapter_visual_plan_v2 数据已删除"
        )))
    }
}

const UP_SQL: &str = r#"
UPDATE chapter_shooting_script
SET visual_plan_id = NULL,
    visual_plan_version = NULL,
    visual_plan_sha256 = NULL
WHERE visual_plan_id IN (
    SELECT id
    FROM chapter_visual_plan
    WHERE contract_version = 'chapter_visual_plan_v2'
);

UPDATE source_snapshot_chapter
SET visual_plan_id = NULL,
    visual_plan_version = NULL,
    visual_plan_sha256 = NULL
WHERE visual_plan_id IN (
    SELECT id
    FROM chapter_visual_plan
    WHERE contract_version = 'chapter_visual_plan_v2'
);

DELETE FROM chapter_visual_plan
WHERE contract_version = 'chapter_visual_plan_v2';
"#;

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn clears_references_before_deleting_retired_plans() {
        let delete = UP_SQL
            .find("DELETE FROM chapter_visual_plan")
            .expect("retired plans must be deleted");
        assert!(
            UP_SQL[..delete].contains("UPDATE chapter_shooting_script")
                && UP_SQL[..delete].contains("UPDATE source_snapshot_chapter"),
            "plan references must be cleared before the rows are deleted"
        );
        assert!(UP_SQL.contains("contract_version = 'chapter_visual_plan_v2'"));
    }
}
