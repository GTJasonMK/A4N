pub(super) fn is_editor_tool(name: &str) -> bool {
    matches!(
        name,
        "inspect_editor_target" | "read_editor_draft" | "save_editor_draft" | "control_editor_view"
    )
}

pub(super) fn needs_editor(name: &str, arguments: &serde_json::Value) -> bool {
    if is_editor_tool(name) {
        return true;
    }
    if !matches!(
        name,
        "edit_chapter_visual_plan_draft" | "save_chapter_visual_plan_draft"
    ) {
        return false;
    }
    let input = &arguments["input"];
    // An explicit persisted base is independent of any window. A requested
    // editor revision still targets that exact live view and cannot fall back.
    !input["editor_revision"].is_null()
        || input["record_id"].as_i64().is_none_or(|id| id <= 0)
        || input["expected_revision"]
            .as_i64()
            .is_none_or(|revision| revision <= 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn saved_base_can_continue_without_a_view_but_drafts_cannot() {
        let saved = json!({"input": {"record_id": 1, "expected_revision": 2}});
        assert!(!needs_editor("edit_chapter_visual_plan_draft", &saved));
        assert!(needs_editor(
            "save_chapter_visual_plan_draft",
            &json!({"input": {}})
        ));
        assert!(needs_editor("read_editor_draft", &saved));
        assert!(needs_editor(
            "edit_chapter_visual_plan_draft",
            &json!({"input": {"record_id": 1, "expected_revision": 2, "editor_revision": "view-v1"}})
        ));
    }
}
