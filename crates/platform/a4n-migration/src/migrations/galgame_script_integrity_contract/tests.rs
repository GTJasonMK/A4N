use super::*;

#[test]
fn line_family_trigger_enforces_event_cg_presentation_contract() {
    for expected in [
        "presentation_json.cg 必须是布尔值",
        "presentation_json.cg_importance 必须是字符串",
        "presentation_json.cg_importance 无效",
        "启用事件 CG 时必须提供 cg_note",
        "dialogue/monologue 行必须显式提供 pose",
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
