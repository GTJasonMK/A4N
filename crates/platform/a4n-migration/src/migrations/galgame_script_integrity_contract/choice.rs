pub(crate) const GALGAME_CHOICE_FAMILY_FUNCTION_SQL: &str = r#"
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
