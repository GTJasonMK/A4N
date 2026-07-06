//! Enforce Galgame script table contracts at the database boundary.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
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

const GALGAME_SCRIPT_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_script_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind_value TEXT;
    source_mode_value TEXT;
    project_root_project_id BIGINT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.source_mode,
           project_row.root_project_id,
           project_row.source_snapshot_id
      INTO project_kind_value,
           source_mode_value,
           project_root_project_id,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind_value IS NULL THEN
        RAISE EXCEPTION 'Galgame 剧本 project_id 不存在';
    END IF;
    IF project_kind_value <> 'galgame' OR source_mode_value <> 'derived' THEN
        RAISE EXCEPTION 'Galgame 剧本 project_id 必须指向 galgame 派生项目';
    END IF;
    IF project_root_project_id IS DISTINCT FROM NEW.root_project_id
       OR project_source_snapshot_id IS DISTINCT FROM NEW.source_snapshot_id THEN
        RAISE EXCEPTION 'Galgame 剧本必须匹配派生项目的根项目和源快照';
    END IF;
    IF NOT EXISTS (
        SELECT 1
        FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id
          AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION 'Galgame 剧本 source_snapshot_id 必须属于 root_project_id';
    END IF;
    IF NEW.status NOT IN ('draft', 'generating', 'ready') THEN
        RAISE EXCEPTION 'Galgame 剧本 status 无效';
    END IF;

    RETURN NEW;
END;
$$
"#;

const GALGAME_SCENE_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    scene_row galgame_scene%ROWTYPE;
    script_root_project_id BIGINT;
BEGIN
    SELECT * INTO scene_row
    FROM galgame_scene
    WHERE id = NEW.id;

    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    IF scene_row.scene_index <= 0 THEN
        RAISE EXCEPTION 'Galgame 场 scene_index 必须是正整数';
    END IF;
    IF BTRIM(scene_row.title) = '' THEN
        RAISE EXCEPTION 'Galgame 场 title 不能为空';
    END IF;
    IF scene_row.entry_kind NOT IN ('main', 'branch') THEN
        RAISE EXCEPTION 'Galgame 场 entry_kind 无效';
    END IF;
    IF jsonb_typeof(scene_row.participants_json::jsonb) IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION 'Galgame 场 participants_json 必须是数组';
    END IF;
    SELECT script.root_project_id
      INTO script_root_project_id
    FROM galgame_script script
    JOIN source_snapshot_chapter chapter
      ON chapter.id = scene_row.source_snapshot_chapter_id
    WHERE script.id = scene_row.script_id
      AND chapter.snapshot_id = script.source_snapshot_id
      AND chapter.root_project_id = script.root_project_id;
    IF script_root_project_id IS NULL THEN
        RAISE EXCEPTION 'Galgame 场 source_snapshot_chapter_id 必须属于剧本源快照';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM jsonb_array_elements(scene_row.participants_json::jsonb) participant(item)
        WHERE jsonb_typeof(participant.item) IS DISTINCT FROM 'object'
    ) THEN
        RAISE EXCEPTION 'Galgame 场 participants_json 项必须是对象';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM jsonb_array_elements(scene_row.participants_json::jsonb) participant(item)
        WHERE participant.item ? 'character_profile_id'
          AND CASE
              WHEN jsonb_typeof(participant.item -> 'character_profile_id') IS DISTINCT FROM 'number' THEN TRUE
              WHEN NOT ((participant.item ->> 'character_profile_id') ~ '^[0-9]+$') THEN TRUE
              WHEN (participant.item ->> 'character_profile_id')::BIGINT <= 0 THEN TRUE
              ELSE FALSE
          END
    ) THEN
        RAISE EXCEPTION 'Galgame 场 participants_json.character_profile_id 必须是正整数';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM jsonb_array_elements(scene_row.participants_json::jsonb) participant(item)
        WHERE participant.item ? 'character_profile_id'
          AND NOT EXISTS (
              SELECT 1
              FROM protagonist_profile profile
              WHERE profile.id = (participant.item ->> 'character_profile_id')::BIGINT
                AND profile.project_id = script_root_project_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 场 participants_json.character_profile_id 必须属于剧本根项目';
    END IF;

    RETURN NEW;
END;
$$
"#;

const GALGAME_SCENE_LINK_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_link_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    affected_script_ids BIGINT[];
    check_script_id BIGINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        affected_script_ids := ARRAY[NEW.script_id];
    ELSIF TG_OP = 'DELETE' THEN
        affected_script_ids := ARRAY[OLD.script_id];
    ELSIF OLD.script_id IS DISTINCT FROM NEW.script_id THEN
        affected_script_ids := ARRAY[OLD.script_id, NEW.script_id];
    ELSE
        affected_script_ids := ARRAY[NEW.script_id];
    END IF;

    FOREACH check_script_id IN ARRAY affected_script_ids LOOP
        IF NOT EXISTS (
            SELECT 1
            FROM galgame_scene scene
            WHERE scene.script_id = check_script_id
        ) THEN
            CONTINUE;
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            WHERE scene.script_id = check_script_id
              AND scene.next_scene_id = scene.id
        ) THEN
            RAISE EXCEPTION 'Galgame 场 next_scene_id 不能指向自身';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            LEFT JOIN galgame_scene target
              ON target.id = scene.next_scene_id
             AND target.script_id = scene.script_id
            WHERE scene.script_id = check_script_id
              AND scene.next_scene_id IS NOT NULL
              AND target.id IS NULL
        ) THEN
            RAISE EXCEPTION 'Galgame 场 next_scene_id 必须属于同一剧本';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            WHERE scene.script_id = check_script_id
              AND scene.entry_kind = 'branch'
              AND scene.next_scene_id IS NULL
        ) THEN
            RAISE EXCEPTION 'Galgame branch 场缺少 next_scene_id';
        END IF;
        IF EXISTS (
            SELECT 1
            FROM galgame_scene scene
            JOIN galgame_scene target
              ON target.id = scene.next_scene_id
             AND target.script_id = scene.script_id
            WHERE scene.script_id = check_script_id
              AND scene.entry_kind = 'branch'
              AND target.entry_kind <> 'main'
        ) THEN
            RAISE EXCEPTION 'Galgame branch 场 next_scene_id 必须指向主线场';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#;

const GALGAME_LINE_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    line_row galgame_line%ROWTYPE;
    script_root_project_id BIGINT;
    script_source_snapshot_id BIGINT;
    scene_source_snapshot_chapter_id BIGINT;
    scene_source_chapter_id BIGINT;
    choice_count BIGINT;
    choice_distinct_index_count BIGINT;
    choice_min_option_index INTEGER;
    choice_max_option_index INTEGER;
BEGIN
    SELECT * INTO line_row
    FROM galgame_line
    WHERE id = NEW.id;

    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    SELECT script.root_project_id,
           script.source_snapshot_id,
           scene.source_snapshot_chapter_id,
           chapter.source_chapter_id
      INTO script_root_project_id,
           script_source_snapshot_id,
           scene_source_snapshot_chapter_id,
           scene_source_chapter_id
    FROM galgame_scene scene
    JOIN galgame_script script ON script.id = scene.script_id
    JOIN source_snapshot_chapter chapter ON chapter.id = scene.source_snapshot_chapter_id
    WHERE scene.id = line_row.scene_id;

    IF script_root_project_id IS NULL THEN
        RAISE EXCEPTION 'Galgame 行 scene_id 不存在';
    END IF;
    IF line_row.line_index <= 0 THEN
        RAISE EXCEPTION 'Galgame 行 line_index 必须是正整数';
    END IF;
    IF line_row.line_type NOT IN ('dialogue', 'narration', 'monologue', 'stage_direction', 'choice') THEN
        RAISE EXCEPTION 'Galgame 行 line_type 无效';
    END IF;
    IF BTRIM(line_row.text) = '' THEN
        RAISE EXCEPTION 'Galgame 行 text 不能为空';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb) IS DISTINCT FROM 'object' THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json 必须是对象';
    END IF;
    IF line_row.presentation_json::jsonb ? 'cg'
       AND jsonb_typeof(line_row.presentation_json::jsonb -> 'cg') NOT IN ('boolean', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg 必须是布尔值';
    END IF;
    IF line_row.presentation_json::jsonb ? 'cg_importance'
       AND jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_importance') NOT IN ('string', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg_importance 必须是字符串';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_importance') = 'string'
       AND BTRIM(line_row.presentation_json::jsonb ->> 'cg_importance') <> ''
       AND BTRIM(line_row.presentation_json::jsonb ->> 'cg_importance') NOT IN ('normal', 'high', 'critical') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg_importance 无效';
    END IF;
    IF (line_row.presentation_json::jsonb ->> 'cg') = 'true'
       AND COALESCE(
           CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_note') = 'string'
                THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_note'), '') END,
           CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_prompt') = 'string'
                THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_prompt'), '') END,
           CASE WHEN jsonb_typeof(line_row.presentation_json::jsonb -> 'event_cg_note') = 'string'
                THEN NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'event_cg_note'), '') END
       ) IS NULL THEN
        RAISE EXCEPTION 'Galgame 行启用事件 CG 时必须提供 cg_note';
    END IF;
    IF line_row.expression IS NOT NULL
       AND line_row.expression NOT IN ('neutral', 'smile', 'sad', 'angry', 'surprised', 'shy', 'serious') THEN
        RAISE EXCEPTION 'Galgame 行 expression 无效';
    END IF;
    IF line_row.position IS NOT NULL
       AND line_row.position NOT IN ('left', 'center', 'right') THEN
        RAISE EXCEPTION 'Galgame 行 position 无效';
    END IF;

    IF line_row.line_type IN ('dialogue', 'monologue') THEN
        IF line_row.speaker_profile_id IS NULL THEN
            RAISE EXCEPTION 'dialogue/monologue 行必须绑定 speaker_profile_id';
        END IF;
        IF line_row.expression IS NULL OR BTRIM(line_row.expression) = '' THEN
            RAISE EXCEPTION 'dialogue/monologue 行必须显式提供 expression';
        END IF;
    ELSIF line_row.speaker_profile_id IS NOT NULL THEN
        RAISE EXCEPTION '非 dialogue/monologue 行不能绑定 speaker_profile_id';
    END IF;

    IF line_row.speaker_profile_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM protagonist_profile profile
        WHERE profile.id = line_row.speaker_profile_id
          AND profile.project_id = script_root_project_id
    ) THEN
        RAISE EXCEPTION 'Galgame 行 speaker_profile_id 必须属于剧本根项目';
    END IF;

    IF line_row.text_anchor_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM text_anchor anchor
        WHERE anchor.id = line_row.text_anchor_id
          AND anchor.project_id = script_root_project_id
          AND anchor.source_snapshot_id = script_source_snapshot_id
          AND anchor.source_snapshot_chapter_id = scene_source_snapshot_chapter_id
          AND (
              anchor.source_chapter_id IS NULL
              OR anchor.source_chapter_id = scene_source_chapter_id
          )
    ) THEN
        RAISE EXCEPTION 'Galgame 行 text_anchor_id 必须属于所在场源快照章节';
    END IF;

    IF line_row.line_type <> 'choice' AND EXISTS (
        SELECT 1
        FROM galgame_choice choice_row
        WHERE choice_row.line_id = line_row.id
    ) THEN
        RAISE EXCEPTION '非 choice Galgame 行不能保留 galgame_choice 选项';
    END IF;
    IF line_row.line_type = 'choice' THEN
        SELECT COUNT(*),
               COUNT(DISTINCT choice_row.option_index),
               MIN(choice_row.option_index),
               MAX(choice_row.option_index)
          INTO choice_count,
               choice_distinct_index_count,
               choice_min_option_index,
               choice_max_option_index
        FROM galgame_choice choice_row
        WHERE choice_row.line_id = line_row.id;
        IF choice_count NOT BETWEEN 2 AND 3 THEN
            RAISE EXCEPTION 'Galgame choice 行选项数量必须是 2 到 3 个';
        END IF;
        IF choice_distinct_index_count <> choice_count
           OR choice_min_option_index <> 1
           OR choice_max_option_index <> choice_count THEN
            RAISE EXCEPTION 'Galgame 选项 option_index 必须从 1 连续编号';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ 
"#;

const GALGAME_LINE_ORDER_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_order_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    affected_scene_ids BIGINT[];
    check_scene_id BIGINT;
    line_count BIGINT;
    line_distinct_index_count BIGINT;
    line_min_index INTEGER;
    line_max_index INTEGER;
BEGIN
    IF TG_OP = 'INSERT' THEN
        affected_scene_ids := ARRAY[NEW.scene_id];
    ELSIF TG_OP = 'DELETE' THEN
        affected_scene_ids := ARRAY[OLD.scene_id];
    ELSIF OLD.scene_id IS DISTINCT FROM NEW.scene_id THEN
        affected_scene_ids := ARRAY[OLD.scene_id, NEW.scene_id];
    ELSE
        affected_scene_ids := ARRAY[NEW.scene_id];
    END IF;

    FOREACH check_scene_id IN ARRAY affected_scene_ids LOOP
        SELECT COUNT(*),
               COUNT(DISTINCT line.line_index),
               MIN(line.line_index),
               MAX(line.line_index)
          INTO line_count,
               line_distinct_index_count,
               line_min_index,
               line_max_index
        FROM galgame_line line
        WHERE line.scene_id = check_scene_id;

        IF line_count = 0 THEN
            CONTINUE;
        END IF;
        IF line_distinct_index_count <> line_count
           OR line_min_index <> 1
           OR line_max_index <> line_count THEN
            RAISE EXCEPTION 'Galgame 行 line_index 必须在同一场内从 1 连续编号且不重复';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#;

const GALGAME_CHOICE_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_choice_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    line_type_value TEXT;
    line_script_id BIGINT;
    target_entry_kind TEXT;
    target_next_scene_id BIGINT;
    return_entry_kind TEXT;
    affected_line_ids BIGINT[];
    check_line_id BIGINT;
    choice_count BIGINT;
    choice_distinct_index_count BIGINT;
    choice_min_option_index INTEGER;
    choice_max_option_index INTEGER;
BEGIN
    IF TG_OP IN ('INSERT', 'UPDATE') THEN
        SELECT line.line_type,
               scene.script_id
          INTO line_type_value,
               line_script_id
        FROM galgame_line line
        JOIN galgame_scene scene ON scene.id = line.scene_id
        WHERE line.id = NEW.line_id;

        IF line_type_value IS NULL THEN
            RAISE EXCEPTION 'Galgame 选项 line_id 不存在';
        END IF;
        IF line_type_value <> 'choice' THEN
            RAISE EXCEPTION 'Galgame 选项只能挂在 choice 行下';
        END IF;
        IF NEW.option_index <= 0 THEN
            RAISE EXCEPTION 'Galgame 选项 option_index 必须是正整数';
        END IF;
        IF BTRIM(NEW.option_text) = '' THEN
            RAISE EXCEPTION 'Galgame 选项 option_text 不能为空';
        END IF;
        IF NEW.target_scene_id IS NULL THEN
            RAISE EXCEPTION 'Galgame 选项 target_scene_id 不能为空';
        END IF;

        SELECT target.entry_kind,
               target.next_scene_id
          INTO target_entry_kind,
               target_next_scene_id
        FROM galgame_scene target
        WHERE target.id = NEW.target_scene_id
          AND target.script_id = line_script_id;

        IF target_entry_kind IS NULL THEN
            RAISE EXCEPTION 'Galgame 选项 target_scene_id 必须属于同一剧本';
        END IF;

        IF target_entry_kind = 'branch' THEN
            IF target_next_scene_id IS NULL THEN
                RAISE EXCEPTION 'Galgame 选项 target_scene_id 指向未回收敛支线';
            END IF;
            SELECT return_scene.entry_kind INTO return_entry_kind
            FROM galgame_scene return_scene
            WHERE return_scene.id = target_next_scene_id
              AND return_scene.script_id = line_script_id;
            IF return_entry_kind IS DISTINCT FROM 'main' THEN
                RAISE EXCEPTION 'Galgame 选项支线回收敛目标必须是同剧本主线场';
            END IF;
        END IF;
    END IF;

    IF TG_OP = 'INSERT' THEN
        affected_line_ids := ARRAY[NEW.line_id];
    ELSIF TG_OP = 'DELETE' THEN
        affected_line_ids := ARRAY[OLD.line_id];
    ELSIF OLD.line_id IS DISTINCT FROM NEW.line_id THEN
        affected_line_ids := ARRAY[OLD.line_id, NEW.line_id];
    ELSE
        affected_line_ids := ARRAY[NEW.line_id];
    END IF;

    FOREACH check_line_id IN ARRAY affected_line_ids LOOP
        SELECT line.line_type INTO line_type_value
        FROM galgame_line line
        WHERE line.id = check_line_id;

        IF line_type_value IS DISTINCT FROM 'choice' THEN
            CONTINUE;
        END IF;

        SELECT COUNT(*),
               COUNT(DISTINCT choice_row.option_index),
               MIN(choice_row.option_index),
               MAX(choice_row.option_index)
          INTO choice_count,
               choice_distinct_index_count,
               choice_min_option_index,
               choice_max_option_index
        FROM galgame_choice choice_row
        WHERE choice_row.line_id = check_line_id;

        IF choice_count NOT BETWEEN 2 AND 3 THEN
            RAISE EXCEPTION 'Galgame choice 行选项数量必须是 2 到 3 个';
        END IF;
        IF choice_distinct_index_count <> choice_count
           OR choice_min_option_index <> 1
           OR choice_max_option_index <> choice_count THEN
            RAISE EXCEPTION 'Galgame 选项 option_index 必须从 1 连续编号';
        END IF;
    END LOOP;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$
"#;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_family_trigger_enforces_event_cg_presentation_contract() {
        for expected in [
            "presentation_json.cg 必须是布尔值",
            "presentation_json.cg_importance 必须是字符串",
            "presentation_json.cg_importance 无效",
            "启用事件 CG 时必须提供 cg_note",
        ] {
            assert!(
                GALGAME_LINE_FAMILY_FUNCTION_SQL.contains(expected),
                "missing Galgame line trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn scene_family_trigger_enforces_participants_contract() {
        for expected in [
            "participants_json 项必须是对象",
            "participants_json.character_profile_id 必须是正整数",
            "participants_json.character_profile_id 必须属于剧本根项目",
            "jsonb_array_elements(scene_row.participants_json::jsonb)",
            "protagonist_profile profile",
        ] {
            assert!(
                GALGAME_SCENE_FAMILY_FUNCTION_SQL.contains(expected),
                "missing Galgame scene participants trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn triggers_enforce_scene_link_contract() {
        for expected in [
            "CREATE OR REPLACE FUNCTION a4n_validate_galgame_scene_link_family()",
            "Galgame 场 next_scene_id 不能指向自身",
            "Galgame 场 next_scene_id 必须属于同一剧本",
            "Galgame branch 场缺少 next_scene_id",
            "Galgame branch 场 next_scene_id 必须指向主线场",
            "CREATE CONSTRAINT TRIGGER trg_galgame_scene_link_family_delete",
            "AFTER DELETE ON galgame_scene",
        ] {
            assert!(
                GALGAME_SCENE_LINK_FAMILY_FUNCTION_SQL.contains(expected)
                    || GALGAME_SCRIPT_INTEGRITY_UP_SQL
                        .iter()
                        .any(|sql| sql.contains(expected)),
                "missing Galgame scene link trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn triggers_enforce_choice_group_contract() {
        for expected in [
            "Galgame choice 行选项数量必须是 2 到 3 个",
            "Galgame 选项 option_index 必须从 1 连续编号",
            "CREATE CONSTRAINT TRIGGER trg_galgame_choice_family_delete",
            "AFTER DELETE ON galgame_choice",
        ] {
            assert!(
                GALGAME_LINE_FAMILY_FUNCTION_SQL.contains(expected)
                    || GALGAME_CHOICE_FAMILY_FUNCTION_SQL.contains(expected)
                    || GALGAME_SCRIPT_INTEGRITY_UP_SQL
                        .iter()
                        .any(|sql| sql.contains(expected)),
                "missing Galgame choice trigger contract: {expected}"
            );
        }
    }

    #[test]
    fn triggers_enforce_line_order_group_contract() {
        for expected in [
            "CREATE OR REPLACE FUNCTION a4n_validate_galgame_line_order_family()",
            "Galgame 行 line_index 必须在同一场内从 1 连续编号且不重复",
            "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_insert",
            "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_update",
            "CREATE CONSTRAINT TRIGGER trg_galgame_line_order_family_delete",
            "AFTER DELETE ON galgame_line",
        ] {
            assert!(
                GALGAME_LINE_ORDER_FAMILY_FUNCTION_SQL.contains(expected)
                    || GALGAME_SCRIPT_INTEGRITY_UP_SQL
                        .iter()
                        .any(|sql| sql.contains(expected)),
                "missing Galgame line order trigger contract: {expected}"
            );
        }
    }
}
