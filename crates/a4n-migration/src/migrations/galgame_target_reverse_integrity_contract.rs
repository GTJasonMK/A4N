//! Enforce reverse integrity from Galgame script targets back to generation/media graphs.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
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

pub(crate) const GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_target_reverse_integrity()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    affected_project_ids BIGINT[];
    check_project_id BIGINT;
BEGIN
    IF TG_TABLE_NAME = 'galgame_script' THEN
        IF TG_OP = 'DELETE' THEN
            affected_project_ids := ARRAY[OLD.project_id];
        ELSE
            SELECT ARRAY_AGG(DISTINCT project_id)
              INTO affected_project_ids
            FROM (
                SELECT OLD.project_id AS project_id
                UNION ALL
                SELECT NEW.project_id AS project_id
            ) project_ids
            WHERE project_id IS NOT NULL;
        END IF;
    ELSIF TG_TABLE_NAME = 'galgame_scene' THEN
        SELECT ARRAY_AGG(DISTINCT script.project_id)
          INTO affected_project_ids
        FROM galgame_script script
        WHERE script.id IN (
            CASE WHEN TG_OP = 'DELETE' THEN OLD.script_id ELSE NEW.script_id END,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.script_id ELSE NULL END
        );
    ELSIF TG_TABLE_NAME = 'galgame_line' THEN
        SELECT ARRAY_AGG(DISTINCT script.project_id)
          INTO affected_project_ids
        FROM galgame_scene scene
        JOIN galgame_script script ON script.id = scene.script_id
        WHERE scene.id IN (
            CASE WHEN TG_OP = 'DELETE' THEN OLD.scene_id ELSE NEW.scene_id END,
            CASE WHEN TG_OP = 'UPDATE' THEN OLD.scene_id ELSE NULL END
        );
    ELSE
        affected_project_ids := ARRAY[]::BIGINT[];
    END IF;

    IF affected_project_ids IS NULL OR CARDINALITY(affected_project_ids) = 0 THEN
        IF TG_OP = 'DELETE' THEN
            RETURN OLD;
        END IF;
        RETURN NEW;
    END IF;

    FOREACH check_project_id IN ARRAY affected_project_ids LOOP
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            LEFT JOIN galgame_script script ON script.id = scene.script_id
            LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
            WHERE script.project_id = check_project_id
              AND (
                  chapter.id IS NULL
                  OR chapter.snapshot_id IS DISTINCT FROM script.source_snapshot_id
                  OR chapter.root_project_id IS DISTINCT FROM script.root_project_id
              )
        ) THEN
            RAISE EXCEPTION 'Galgame 场更新后必须仍属于剧本源快照章节';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM galgame_line line_row
            LEFT JOIN galgame_scene scene ON scene.id = line_row.scene_id
            LEFT JOIN galgame_script script ON script.id = scene.script_id
            LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
            LEFT JOIN text_anchor anchor ON anchor.id = line_row.text_anchor_id
            LEFT JOIN protagonist_profile profile ON profile.id = line_row.speaker_profile_id
            WHERE script.project_id = check_project_id
              AND (
                  scene.id IS NULL
                  OR chapter.id IS NULL
                  OR (
                      line_row.text_anchor_id IS NOT NULL
                      AND (
                          anchor.id IS NULL
                          OR anchor.project_id IS DISTINCT FROM script.root_project_id
                          OR anchor.source_snapshot_id IS DISTINCT FROM script.source_snapshot_id
                          OR anchor.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
                          OR (
                              anchor.source_chapter_id IS NOT NULL
                              AND anchor.source_chapter_id IS DISTINCT FROM chapter.source_chapter_id
                          )
                      )
                  )
                  OR (
                      line_row.speaker_profile_id IS NOT NULL
                      AND (
                          profile.id IS NULL
                          OR profile.project_id IS DISTINCT FROM script.root_project_id
                      )
                  )
              )
        ) THEN
            RAISE EXCEPTION 'Galgame 行更新后必须仍匹配场章节、文本锚点和说话人档案';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM generation_spec spec
            LEFT JOIN galgame_scene scene ON scene.id = spec.target_entity_id
            LEFT JOIN galgame_script script ON script.id = scene.script_id
            LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
            WHERE spec.project_id = check_project_id
              AND spec.target_kind = 'galgame_background'
              AND spec.target_entity_kind = 'galgame_scene'
              AND (
                  scene.id IS NULL
                  OR script.project_id IS DISTINCT FROM spec.project_id
                  OR script.root_project_id IS DISTINCT FROM spec.root_project_id
                  OR script.source_snapshot_id IS DISTINCT FROM spec.source_snapshot_id
                  OR chapter.id IS NULL
                  OR spec.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
                  OR spec.source_chapter_id IS DISTINCT FROM chapter.source_chapter_id
                  OR spec.params_json::jsonb #>> '{scene,id}' IS DISTINCT FROM spec.target_entity_id::TEXT
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{scene,title}'), '') IS DISTINCT FROM NULLIF(BTRIM(scene.title), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{scene,location}'), '') IS DISTINCT FROM NULLIF(BTRIM(scene.location), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{scene,time_of_day}'), '') IS DISTINCT FROM NULLIF(BTRIM(scene.time_of_day), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{scene,background_note}'), '') IS DISTINCT FROM NULLIF(BTRIM(scene.background_note), '')
                  OR spec.params_json::jsonb #>> '{source,source_snapshot_chapter_id}' IS DISTINCT FROM spec.source_snapshot_chapter_id::TEXT
                  OR spec.params_json::jsonb #>> '{source,source_chapter_id}' IS DISTINCT FROM spec.source_chapter_id::TEXT
              )
        ) THEN
            RAISE EXCEPTION 'Galgame 背景生成意图必须随目标场更新或清理';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM generation_spec spec
            LEFT JOIN galgame_line line_row ON line_row.id = spec.target_entity_id
            LEFT JOIN galgame_scene scene ON scene.id = line_row.scene_id
            LEFT JOIN galgame_script script ON script.id = scene.script_id
            LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
            WHERE spec.project_id = check_project_id
              AND spec.target_kind = 'galgame_cg'
              AND spec.target_entity_kind = 'galgame_line'
              AND (
                  line_row.id IS NULL
                  OR scene.id IS NULL
                  OR script.project_id IS DISTINCT FROM spec.project_id
                  OR script.root_project_id IS DISTINCT FROM spec.root_project_id
                  OR script.source_snapshot_id IS DISTINCT FROM spec.source_snapshot_id
                  OR chapter.id IS NULL
                  OR spec.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
                  OR spec.source_chapter_id IS DISTINCT FROM chapter.source_chapter_id
                  OR spec.params_json::jsonb #>> '{line,id}' IS DISTINCT FROM spec.target_entity_id::TEXT
                  OR spec.params_json::jsonb #>> '{line,line_index}' IS DISTINCT FROM line_row.line_index::TEXT
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{line,line_type}'), '') IS DISTINCT FROM NULLIF(BTRIM(line_row.line_type), '')
                  OR spec.params_json::jsonb #>> '{line,speaker_profile_id}' IS DISTINCT FROM line_row.speaker_profile_id::TEXT
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{line,text}'), '') IS DISTINCT FROM NULLIF(BTRIM(line_row.text), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{line,expression}'), '') IS DISTINCT FROM NULLIF(BTRIM(line_row.expression), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{line,pose}'), '') IS DISTINCT FROM NULLIF(BTRIM(line_row.pose), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{line,position}'), '') IS DISTINCT FROM NULLIF(BTRIM(line_row.position), '')
                  OR spec.params_json::jsonb #>> '{scene,id}' IS DISTINCT FROM scene.id::TEXT
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{scene,title}'), '') IS DISTINCT FROM NULLIF(BTRIM(scene.title), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{scene,location}'), '') IS DISTINCT FROM NULLIF(BTRIM(scene.location), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb #>> '{scene,time_of_day}'), '') IS DISTINCT FROM NULLIF(BTRIM(scene.time_of_day), '')
                  OR NULLIF(BTRIM(spec.params_json::jsonb ->> 'cg_note'), '') IS DISTINCT FROM COALESCE(
                      CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_note') = 'string'
                           THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_note'), '') END,
                      CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_prompt') = 'string'
                           THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_prompt'), '') END,
                      CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'event_cg_note') = 'string'
                           THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'event_cg_note'), '') END
                  )
                  OR COALESCE(NULLIF(BTRIM(spec.params_json::jsonb ->> 'cg_importance'), ''), 'normal') IS DISTINCT FROM COALESCE(NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_importance'), ''), 'normal')
              )
        ) THEN
            RAISE EXCEPTION 'Galgame CG 生成意图必须随目标行更新或清理';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM generation_spec spec
            WHERE spec.project_id = check_project_id
              AND spec.target_kind = 'portrait_expression'
              AND spec.target_entity_kind = 'character_profile'
              AND (
                  jsonb_typeof(spec.params_json::jsonb -> 'source_line_ids') IS DISTINCT FROM 'array'
                  OR CASE
                      WHEN jsonb_typeof(spec.params_json::jsonb -> 'source_line_ids') = 'array'
                      THEN jsonb_array_length(spec.params_json::jsonb -> 'source_line_ids') = 0
                      ELSE TRUE
                  END
                  OR CASE
                      WHEN jsonb_typeof(spec.params_json::jsonb -> 'source_line_ids') = 'array' THEN EXISTS (
                          SELECT 1
                          FROM jsonb_array_elements(spec.params_json::jsonb -> 'source_line_ids') AS elem(value)
                          WHERE jsonb_typeof(elem.value) IS DISTINCT FROM 'number'
                             OR (elem.value #>> '{}') !~ '^[0-9]+$'
                             OR (elem.value #>> '{}')::BIGINT <= 0
                      )
                      ELSE TRUE
                  END
                  OR CASE
                      WHEN jsonb_typeof(spec.params_json::jsonb -> 'source_line_ids') = 'array' THEN EXISTS (
                          WITH source_ids AS (
                              SELECT (elem.value #>> '{}')::BIGINT AS line_id
                              FROM jsonb_array_elements(spec.params_json::jsonb -> 'source_line_ids') AS elem(value)
                              WHERE jsonb_typeof(elem.value) = 'number'
                                AND (elem.value #>> '{}') ~ '^[0-9]+$'
                          )
                          SELECT 1
                          FROM source_ids source_id
                          LEFT JOIN galgame_line line_row ON line_row.id = source_id.line_id
                          LEFT JOIN galgame_scene scene ON scene.id = line_row.scene_id
                          LEFT JOIN galgame_script script ON script.id = scene.script_id
                          LEFT JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
                          WHERE line_row.id IS NULL
                             OR line_row.speaker_profile_id IS DISTINCT FROM spec.target_entity_id
                             OR NULLIF(BTRIM(COALESCE(line_row.expression, '')), '') IS DISTINCT FROM NULLIF(BTRIM(spec.params_json::jsonb ->> 'expression'), '')
                             OR COALESCE(NULLIF(BTRIM(line_row.pose), ''), '') IS DISTINCT FROM COALESCE(NULLIF(BTRIM(spec.params_json::jsonb ->> 'pose'), ''), '')
                             OR script.project_id IS DISTINCT FROM spec.project_id
                             OR script.root_project_id IS DISTINCT FROM spec.root_project_id
                             OR script.source_snapshot_id IS DISTINCT FROM spec.source_snapshot_id
                             OR chapter.snapshot_id IS DISTINCT FROM spec.source_snapshot_id
                             OR chapter.root_project_id IS DISTINCT FROM spec.root_project_id
                             OR (
                                 NULLIF(BTRIM(line_row.position), '') IS NOT NULL
                                 AND NULLIF(BTRIM(line_row.position), '') NOT IN ('left', 'center', 'right')
                             )
                      )
                      ELSE TRUE
                  END
                  OR jsonb_typeof(spec.params_json::jsonb -> 'position_samples') IS DISTINCT FROM 'array'
                  OR CASE
                      WHEN jsonb_typeof(spec.params_json::jsonb -> 'position_samples') = 'array' THEN EXISTS (
                          SELECT 1
                          FROM jsonb_array_elements(spec.params_json::jsonb -> 'position_samples') AS elem(value)
                          WHERE jsonb_typeof(elem.value) IS DISTINCT FROM 'string'
                             OR NULLIF(BTRIM(elem.value #>> '{}'), '') IS NULL
                             OR BTRIM(elem.value #>> '{}') NOT IN ('left', 'center', 'right')
                      )
                      ELSE TRUE
                  END
                  OR CASE
                      WHEN jsonb_typeof(spec.params_json::jsonb -> 'source_line_ids') = 'array'
                           AND jsonb_typeof(spec.params_json::jsonb -> 'position_samples') = 'array' THEN (
                          SELECT COALESCE(array_agg(DISTINCT sample_position ORDER BY sample_position), ARRAY[]::TEXT[])
                          FROM (
                              SELECT BTRIM(elem.value #>> '{}') AS sample_position
                              FROM jsonb_array_elements(spec.params_json::jsonb -> 'position_samples') AS elem(value)
                              WHERE jsonb_typeof(elem.value) = 'string'
                                AND NULLIF(BTRIM(elem.value #>> '{}'), '') IS NOT NULL
                          ) samples
                      ) IS DISTINCT FROM (
                          WITH source_ids AS (
                              SELECT (elem.value #>> '{}')::BIGINT AS line_id
                              FROM jsonb_array_elements(spec.params_json::jsonb -> 'source_line_ids') AS elem(value)
                              WHERE jsonb_typeof(elem.value) = 'number'
                                AND (elem.value #>> '{}') ~ '^[0-9]+$'
                          )
                          SELECT COALESCE(array_agg(DISTINCT source_position ORDER BY source_position), ARRAY[]::TEXT[])
                          FROM (
                              SELECT NULLIF(BTRIM(line_row.position), '') AS source_position
                              FROM source_ids source_id
                              JOIN galgame_line line_row ON line_row.id = source_id.line_id
                          ) positions
                          WHERE source_position IS NOT NULL
                      )
                      ELSE TRUE
                  END
              )
        ) THEN
            RAISE EXCEPTION 'Galgame 立绘差分生成意图必须随来源行更新或清理';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM media_binding binding
            LEFT JOIN media_asset asset ON asset.id = binding.media_asset_id
            LEFT JOIN galgame_scene scene ON scene.id = binding.target_id
            LEFT JOIN galgame_script script ON script.id = scene.script_id
            WHERE binding.target_kind = 'galgame_scene'
              AND (asset.project_id = check_project_id OR script.project_id = check_project_id)
              AND (
                  binding.role <> 'background'
                  OR asset.id IS NULL
                  OR scene.id IS NULL
                  OR script.project_id IS DISTINCT FROM asset.project_id
                  OR script.root_project_id IS DISTINCT FROM asset.root_project_id
                  OR script.source_snapshot_id IS DISTINCT FROM asset.source_snapshot_id
                  OR asset.media_kind <> 'image'
                  OR asset.status <> 'completed'
                  OR asset.file_path IS NULL
                  OR (
                      asset.source_snapshot_chapter_id IS NOT NULL
                      AND asset.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
                  )
              )
        ) THEN
            RAISE EXCEPTION 'Galgame 场媒体绑定必须随目标场更新或清理';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM media_binding binding
            LEFT JOIN media_asset asset ON asset.id = binding.media_asset_id
            LEFT JOIN galgame_line line_row ON line_row.id = binding.target_id
            LEFT JOIN galgame_scene scene ON scene.id = line_row.scene_id
            LEFT JOIN galgame_script script ON script.id = scene.script_id
            WHERE binding.target_kind = 'galgame_line'
              AND (asset.project_id = check_project_id OR script.project_id = check_project_id)
              AND (
                  binding.role NOT IN ('cg', 'bgm', 'sfx')
                  OR asset.id IS NULL
                  OR line_row.id IS NULL
                  OR scene.id IS NULL
                  OR script.project_id IS DISTINCT FROM asset.project_id
                  OR script.root_project_id IS DISTINCT FROM asset.root_project_id
                  OR script.source_snapshot_id IS DISTINCT FROM asset.source_snapshot_id
                  OR asset.media_kind IS DISTINCT FROM CASE
                      WHEN binding.role IN ('bgm', 'sfx') THEN 'audio'
                      ELSE 'image'
                  END
                  OR asset.status <> 'completed'
                  OR asset.file_path IS NULL
                  OR (
                      asset.source_snapshot_chapter_id IS NOT NULL
                      AND asset.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
                  )
              )
        ) THEN
            RAISE EXCEPTION 'Galgame 行媒体绑定必须随目标行更新或清理';
        END IF;

        IF EXISTS (
            SELECT 1
            FROM media_provenance provenance
            JOIN media_asset asset ON asset.id = provenance.media_asset_id
            LEFT JOIN galgame_line line_row
              ON provenance.request_snapshot_json IS NOT NULL
             AND jsonb_typeof(provenance.request_snapshot_json::jsonb) = 'object'
             AND provenance.request_snapshot_json::jsonb ? 'target_line_id'
             AND jsonb_typeof(provenance.request_snapshot_json::jsonb -> 'target_line_id') = 'number'
             AND (provenance.request_snapshot_json::jsonb ->> 'target_line_id') ~ '^[0-9]+$'
             AND line_row.id = (provenance.request_snapshot_json::jsonb ->> 'target_line_id')::BIGINT
            LEFT JOIN galgame_scene scene ON scene.id = line_row.scene_id
            LEFT JOIN galgame_script script ON script.id = scene.script_id
            WHERE asset.project_id = check_project_id
              AND provenance.request_snapshot_json IS NOT NULL
              AND jsonb_typeof(provenance.request_snapshot_json::jsonb) = 'object'
              AND provenance.request_snapshot_json::jsonb ? 'target_line_id'
              AND (
                  line_row.id IS NULL
                  OR scene.id IS NULL
                  OR script.project_id IS DISTINCT FROM asset.project_id
                  OR script.root_project_id IS DISTINCT FROM asset.root_project_id
                  OR script.source_snapshot_id IS DISTINCT FROM asset.source_snapshot_id
                  OR (
                      asset.source_snapshot_chapter_id IS NOT NULL
                      AND asset.source_snapshot_chapter_id IS DISTINCT FROM scene.source_snapshot_chapter_id
                  )
              )
        ) THEN
            RAISE EXCEPTION 'Galgame 音频媒体来源 target_line_id 必须随目标行更新或清理';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_integrity_rejects_stale_generation_targets() {
        for expected in [
            "Galgame 背景生成意图必须随目标场更新或清理",
            "Galgame CG 生成意图必须随目标行更新或清理",
            "Galgame 立绘差分生成意图必须随来源行更新或清理",
            "source_line_ids",
            "position_samples",
        ] {
            assert!(
                GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL.contains(expected),
                "missing Galgame reverse generation target contract: {expected}"
            );
        }
    }

    #[test]
    fn reverse_integrity_rejects_stale_media_targets() {
        for expected in [
            "Galgame 场媒体绑定必须随目标场更新或清理",
            "Galgame 行媒体绑定必须随目标行更新或清理",
            "Galgame 音频媒体来源 target_line_id 必须随目标行更新或清理",
            "CREATE CONSTRAINT TRIGGER trg_galgame_target_reverse_line_delete",
            "DEFERRABLE INITIALLY DEFERRED",
        ] {
            assert!(
                GALGAME_TARGET_REVERSE_INTEGRITY_FUNCTION_SQL.contains(expected)
                    || GALGAME_TARGET_REVERSE_INTEGRITY_UP_SQL
                        .iter()
                        .any(|sql| sql.contains(expected)),
                "missing Galgame reverse media target contract: {expected}"
            );
        }
    }
}
