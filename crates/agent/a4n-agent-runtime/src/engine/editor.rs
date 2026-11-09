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
    // Only an explicit editor revision binds a saved-plan write to a view.
    // A new saved plan has no record ID yet; malformed bases fail at the owner.
    !arguments["input"]["editor_revision"].is_null()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn saved_plan_writes_only_bind_to_an_explicit_editor_revision() {
        let saved = json!({"input": {"record_id": 1, "expected_revision": 2}});
        assert!(!needs_editor("edit_chapter_visual_plan_draft", &saved));
        assert!(!needs_editor(
            "save_chapter_visual_plan_draft",
            &json!({"input": {}})
        ));
        assert!(needs_editor("read_editor_draft", &saved));
        assert!(needs_editor(
            "edit_chapter_visual_plan_draft",
            &json!({"input": {"record_id": 1, "expected_revision": 2, "editor_revision": "view-v1"}})
        ));
    }

    #[test]
    fn creating_a_saved_plan_does_not_require_an_editor_revision() {
        let input = json!({"input": {
            "project_id": 18, "source_chapter_id": 9,
            "record_id": null, "expected_revision": null,
            "editor_revision": null,
        }});
        assert!(!needs_editor("save_chapter_visual_plan_draft", &input));
        let mut editor_bound = input;
        editor_bound["input"]["editor_revision"] = json!("view-v1");
        assert!(needs_editor(
            "save_chapter_visual_plan_draft",
            &editor_bound
        ));
    }
}
