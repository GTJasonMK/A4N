pub(crate) const GALGAME_SCRIPT_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_galgame_script_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    project_kind_value TEXT;
    project_root_project_id BIGINT;
    project_source_snapshot_id BIGINT;
BEGIN
    SELECT project_row.project_kind,
           project_row.root_project_id,
           project_row.source_snapshot_id
      INTO project_kind_value,
           project_root_project_id,
           project_source_snapshot_id
    FROM novel_project project_row
    WHERE project_row.id = NEW.project_id;

    IF project_kind_value IS NULL THEN
        RAISE EXCEPTION 'Galgame 剧本 project_id 不存在';
    END IF;
    IF project_kind_value <> 'galgame'
       OR project_root_project_id IS NULL
       OR project_source_snapshot_id IS NULL THEN
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

pub(crate) const GALGAME_SCENE_FAMILY_FUNCTION_SQL: &str = r#"
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

pub(crate) const GALGAME_SCENE_LINK_FAMILY_FUNCTION_SQL: &str = r#"
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
