use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

mod profiles;

pub(super) const BACKFILL_FUNCTION_SQL: &str = r#"
-- Legacy main scenes were emitted in chapter/scene order. Only complete,
-- unchanged cohorts with one matching frozen script prove that correspondence.
-- The same conversion runs once at the old-package import boundary.
CREATE FUNCTION a4n_backfill_galgame_scene_sources(target_project_id BIGINT)
RETURNS SETOF BIGINT LANGUAGE SQL AS $$
    WITH mains AS (
        SELECT scene.id, script.project_id, scene.source_snapshot_chapter_id,
               row_number() OVER cohort AS scene_ordinal,
               count(*) OVER chapter AS scene_count,
               min(source.snapshot_id) OVER chapter AS min_snapshot,
               max(source.snapshot_id) OVER chapter AS max_snapshot
        FROM galgame_scene scene
        JOIN galgame_script script ON script.id = scene.script_id
        JOIN source_snapshot_chapter source ON source.id = scene.source_snapshot_chapter_id
        WHERE scene.entry_kind = 'main'
          AND (target_project_id IS NULL OR script.project_id = target_project_id)
        WINDOW chapter AS (PARTITION BY scene.script_id, source.source_chapter_id),
               cohort AS (PARTITION BY scene.script_id, source.source_chapter_id ORDER BY scene.scene_index)
    ), candidates AS (
        SELECT mains.id, shot_scene.value ->> 'scene_identity' AS identity
        FROM mains
        JOIN source_snapshot_chapter source ON source.id = mains.source_snapshot_chapter_id
        JOIN chapter_shooting_script shooting ON shooting.source_snapshot_chapter_id = source.id
            AND shooting.visual_plan_id = source.visual_plan_id
            AND shooting.visual_plan_version = source.visual_plan_version
            AND shooting.visual_plan_sha256 = source.visual_plan_sha256
            AND shooting.source_content_sha256 = source.content_hash
        CROSS JOIN LATERAL jsonb_array_elements(shooting.script_json::jsonb -> 'scenes') shot_scene(value)
        WHERE mains.min_snapshot = mains.max_snapshot
          AND jsonb_array_length(shooting.script_json::jsonb -> 'scenes') = mains.scene_count
          AND shot_scene.value ->> 'scene_index' = mains.scene_ordinal::text
          AND btrim(shot_scene.value ->> 'scene_identity') <> ''
          AND NOT EXISTS (
              SELECT 1 FROM source_sync_conflict conflict
              WHERE conflict.derived_project_id = mains.project_id
                AND conflict.status = 'resolved' AND conflict.resolution_strategy = 'regenerate_scene'
          )
    ), proven AS (
        SELECT id, min(identity) AS identity FROM candidates GROUP BY id HAVING count(*) = 1
    )
    UPDATE galgame_scene scene SET shooting_scene_identity = proven.identity
    FROM proven WHERE scene.id = proven.id AND scene.shooting_scene_identity IS NULL
    RETURNING scene.id
$$;
"#;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        profiles::restore_snapshot_referenced_profiles(manager.get_connection()).await?;
        if !manager
            .has_column("galgame_scene", "shooting_scene_identity")
            .await?
        {
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE galgame_scene ADD COLUMN shooting_scene_identity TEXT",
                )
                .await?;
        }
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE galgame_scene ADD CONSTRAINT ck_galgame_scene_shooting_identity
    CHECK (shooting_scene_identity IS NULL OR btrim(shooting_scene_identity) <> '');
"#,
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(BACKFILL_FUNCTION_SQL)
            .await?;
        manager
            .get_connection()
            .execute_unprepared("SELECT a4n_backfill_galgame_scene_sources(NULL::bigint);")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "DROP FUNCTION a4n_backfill_galgame_scene_sources(BIGINT); ALTER TABLE galgame_scene DROP COLUMN shooting_scene_identity;"
        ).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
