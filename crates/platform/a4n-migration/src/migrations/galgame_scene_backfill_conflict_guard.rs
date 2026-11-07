use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const HISTORICAL_CONFLICT_PREDICATE: &str =
    "conflict.status = 'resolved' AND conflict.resolution_strategy = 'regenerate_scene'";
const CURRENT_CONFLICT_PREDICATE: &str = "conflict.resolution_strategy = 'regenerate_scene'";

pub(super) const CLEAR_CONFLICTED_IDENTITIES_SQL: &str = r#"
UPDATE galgame_scene scene
SET shooting_scene_identity = NULL
FROM galgame_script script
WHERE script.id = scene.script_id
  AND scene.entry_kind = 'main'
  AND scene.shooting_scene_identity IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM source_sync_conflict conflict
      WHERE conflict.derived_project_id = script.project_id
        AND conflict.resolution_strategy = 'regenerate_scene'
  );
"#;

pub(super) fn current_backfill_function_sql() -> Result<String, DbErr> {
    let historical = super::galgame_scene_source_identity::BACKFILL_FUNCTION_SQL;
    if historical.matches(HISTORICAL_CONFLICT_PREDICATE).count() != 1 {
        return Err(DbErr::Custom(
            "galgame_scene_backfill_conflict_guard: unexpected historical backfill contract".into(),
        ));
    }

    Ok(historical
        .replacen("CREATE FUNCTION", "CREATE OR REPLACE FUNCTION", 1)
        .replacen(HISTORICAL_CONFLICT_PREDICATE, CURRENT_CONFLICT_PREDICATE, 1))
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_table("galgame_scene").await?
            || !manager.has_table("source_snapshot_chapter").await?
        {
            return Err(DbErr::Custom(
                "galgame_scene_backfill_conflict_guard: 迁移依赖的数据表缺失".into(),
            ));
        }

        let has_scene_identity = manager
            .has_column("galgame_scene", "shooting_scene_identity")
            .await?;
        let has_source_visual_plan = manager
            .has_column("source_snapshot_chapter", "visual_plan_id")
            .await?;

        if !has_scene_identity && !has_source_visual_plan {
            if manager
                .has_table("media_authoring_migration_receipt")
                .await?
            {
                return Ok(());
            }
            return Err(DbErr::Custom(
                "galgame_scene_backfill_conflict_guard: 迁移所需列缺失，且没有 media_authoring 退役回执"
                    .into(),
            ));
        }
        if has_scene_identity != has_source_visual_plan {
            return Err(DbErr::Custom(
                "galgame_scene_backfill_conflict_guard: 场景身份与来源视觉计划列状态不一致".into(),
            ));
        }

        let connection = manager.get_connection();
        connection
            .execute_unprepared(&current_backfill_function_sql()?)
            .await?;
        connection
            .execute_unprepared(CLEAR_CONFLICTED_IDENTITIES_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "历史画面来源身份一旦失去证据，不能安全恢复".into(),
        ))
    }
}
