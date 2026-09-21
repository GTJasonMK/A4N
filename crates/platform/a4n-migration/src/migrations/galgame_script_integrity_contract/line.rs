pub(crate) const GALGAME_LINE_FAMILY_FUNCTION_SQL: &str = r#"
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
    IF NOT (line_row.presentation_json::jsonb ?& ARRAY[
        'schema_version',
        'visible_cast',
        'enter',
        'exit',
        'focus_character_profile_id',
        'hold',
        'player_knowledge',
        'cg',
        'cg_note',
        'cg_importance',
        'transition',
        'branch',
        'branch_id',
        'returns_to_scene_index',
        'after_beat_ref'
    ]) THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json 缺少 v2 必填字段';
    END IF;
    IF EXISTS (
        SELECT 1
        FROM jsonb_object_keys(line_row.presentation_json::jsonb) AS field(key)
        WHERE field.key <> ALL (ARRAY[
            'schema_version',
            'visible_cast',
            'enter',
            'exit',
            'focus_character_profile_id',
            'hold',
            'player_knowledge',
            'cg',
            'cg_note',
            'cg_importance',
            'transition',
            'branch',
            'branch_id',
            'returns_to_scene_index',
            'after_beat_ref'
        ]::TEXT[])
    ) THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json 包含 v2 未声明字段';
    END IF;
    IF line_row.presentation_json::jsonb ->> 'schema_version'
       IS DISTINCT FROM 'galgame_line_presentation_v2' THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.schema_version 无效';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'visible_cast') IS DISTINCT FROM 'array'
       OR jsonb_typeof(line_row.presentation_json::jsonb -> 'enter') IS DISTINCT FROM 'array'
       OR jsonb_typeof(line_row.presentation_json::jsonb -> 'exit') IS DISTINCT FROM 'array'
       OR jsonb_typeof(line_row.presentation_json::jsonb -> 'hold') IS DISTINCT FROM 'boolean'
       OR jsonb_typeof(line_row.presentation_json::jsonb -> 'player_knowledge') IS DISTINCT FROM 'array'
    THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json v2 核心字段类型无效';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'focus_character_profile_id') NOT IN ('number', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.focus_character_profile_id 必须是正整数或 null';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'focus_character_profile_id') = 'number'
       AND (line_row.presentation_json::jsonb ->> 'focus_character_profile_id') !~ '^[1-9][0-9]*$' THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.focus_character_profile_id 必须是正整数或 null';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'cg') NOT IN ('boolean', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg 必须是布尔值';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_note') NOT IN ('string', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg_note 必须是字符串或 null';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_importance') NOT IN ('string', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg_importance 必须是字符串';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'cg_importance') = 'string'
       AND BTRIM(line_row.presentation_json::jsonb ->> 'cg_importance') NOT IN ('normal', 'high', 'critical') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.cg_importance 无效';
    END IF;
    IF (line_row.presentation_json::jsonb ->> 'cg') = 'true' THEN
        IF NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_note'), '') IS NULL THEN
            RAISE EXCEPTION 'Galgame 行启用事件 CG 时必须提供 cg_note';
        END IF;
        IF NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'cg_importance'), '') IS NULL THEN
            RAISE EXCEPTION 'Galgame 行启用事件 CG 时必须提供 cg_importance';
        END IF;
    ELSIF line_row.presentation_json::jsonb -> 'cg_note' <> 'null'::jsonb
       OR line_row.presentation_json::jsonb -> 'cg_importance' <> 'null'::jsonb THEN
        RAISE EXCEPTION 'Galgame 行未启用事件 CG 时不能保留 cg_note/cg_importance';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'transition') NOT IN ('string', 'null')
       OR (
           jsonb_typeof(line_row.presentation_json::jsonb -> 'transition') = 'string'
           AND line_row.presentation_json::jsonb ->> 'transition' NOT IN ('cut', 'fade', 'dissolve')
       ) THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.transition 无效';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'branch') NOT IN ('boolean', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.branch 必须是布尔值或 null';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'branch_id') NOT IN ('string', 'null')
       OR (
           jsonb_typeof(line_row.presentation_json::jsonb -> 'branch_id') = 'string'
           AND NULLIF(BTRIM(line_row.presentation_json::jsonb ->> 'branch_id'), '') IS NULL
       ) THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.branch_id 必须是非空字符串或 null';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'returns_to_scene_index') NOT IN ('number', 'null')
       OR (
           jsonb_typeof(line_row.presentation_json::jsonb -> 'returns_to_scene_index') = 'number'
           AND (line_row.presentation_json::jsonb ->> 'returns_to_scene_index') !~ '^[1-9][0-9]*$'
       ) THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.returns_to_scene_index 必须是正整数或 null';
    END IF;
    IF jsonb_typeof(line_row.presentation_json::jsonb -> 'after_beat_ref') NOT IN ('object', 'null') THEN
        RAISE EXCEPTION 'Galgame 行 presentation_json.after_beat_ref 必须是对象或 null';
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
        IF line_row.pose IS NULL OR BTRIM(line_row.pose) = '' THEN
            RAISE EXCEPTION 'dialogue/monologue 行必须显式提供 pose';
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

pub(crate) const GALGAME_LINE_ORDER_FAMILY_FUNCTION_SQL: &str = r#"
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
