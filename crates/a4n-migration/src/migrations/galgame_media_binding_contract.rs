//! Enforce Galgame media binding contracts at the database boundary.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N galgame media binding contract migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in GALGAME_MEDIA_BINDING_CONTRACT_UP_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N galgame media binding contract migration only supports PostgreSQL".to_string(),
            ));
        }

        for sql in GALGAME_MEDIA_BINDING_CONTRACT_DOWN_SQL {
            execute_pg_sql(manager, sql).await?;
        }
        Ok(())
    }
}

const GALGAME_MEDIA_BINDING_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_media_binding()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.target_kind NOT IN ('galgame_scene', 'galgame_line') THEN
        RETURN NEW;
    END IF;

    IF NEW.target_kind = 'galgame_scene' AND NEW.role <> 'background' THEN
        RAISE EXCEPTION 'Galgame 场媒体绑定只允许 background 角色';
    END IF;
    IF NEW.target_kind = 'galgame_line' AND NEW.role NOT IN ('cg', 'bgm', 'sfx') THEN
        RAISE EXCEPTION 'Galgame 行媒体绑定只允许 cg/bgm/sfx 角色';
    END IF;

    IF NEW.target_kind = 'galgame_scene' AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN galgame_scene scene ON scene.id = NEW.target_id
        JOIN galgame_script script ON script.id = scene.script_id
        LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
        WHERE asset.id = NEW.media_asset_id
          AND asset.project_id = script.project_id
          AND asset.root_project_id = script.root_project_id
          AND asset.source_snapshot_id = script.source_snapshot_id
          AND asset.media_kind = 'image'
          AND asset.status = 'completed'
          AND asset.file_path IS NOT NULL
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR asset.source_snapshot_chapter_id = scene.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR chapter.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 场媒体绑定必须匹配目标项目、媒体类型和源快照上下文';
    END IF;

    IF NEW.target_kind = 'galgame_line' AND NOT EXISTS (
        SELECT 1
        FROM media_asset asset
        JOIN galgame_line line ON line.id = NEW.target_id
        JOIN galgame_scene scene ON scene.id = line.scene_id
        JOIN galgame_script script ON script.id = scene.script_id
        LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
        WHERE asset.id = NEW.media_asset_id
          AND asset.project_id = script.project_id
          AND asset.root_project_id = script.root_project_id
          AND asset.source_snapshot_id = script.source_snapshot_id
          AND (
              (NEW.role = 'cg' AND asset.media_kind = 'image')
              OR (NEW.role IN ('bgm', 'sfx') AND asset.media_kind = 'audio')
          )
          AND asset.status = 'completed'
          AND asset.file_path IS NOT NULL
          AND (
              asset.source_snapshot_chapter_id IS NULL
              OR asset.source_snapshot_chapter_id = scene.source_snapshot_chapter_id
          )
          AND (
              asset.source_chapter_id IS NULL
              OR chapter.source_chapter_id = asset.source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 行媒体绑定必须匹配目标项目、媒体类型和源快照上下文';
    END IF;

    RETURN NEW;
END;
$$
"#;

const GALGAME_MEDIA_BINDING_CONTRACT_UP_SQL: &[&str] = &[
    GALGAME_MEDIA_BINDING_FUNCTION_SQL,
    "DROP TRIGGER IF EXISTS trg_galgame_media_binding_insert ON media_binding",
    r#"
CREATE TRIGGER trg_galgame_media_binding_insert
BEFORE INSERT ON media_binding
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_media_binding()
"#,
    "DROP TRIGGER IF EXISTS trg_galgame_media_binding_update ON media_binding",
    r#"
CREATE TRIGGER trg_galgame_media_binding_update
BEFORE UPDATE OF media_asset_id, target_kind, target_id, role ON media_binding
FOR EACH ROW EXECUTE FUNCTION a4n_validate_galgame_media_binding()
"#,
];

const GALGAME_MEDIA_BINDING_CONTRACT_DOWN_SQL: &[&str] = &[
    "DROP TRIGGER IF EXISTS trg_galgame_media_binding_update ON media_binding",
    "DROP TRIGGER IF EXISTS trg_galgame_media_binding_insert ON media_binding",
    "DROP FUNCTION IF EXISTS a4n_validate_galgame_media_binding()",
];

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
