pub(super) fn is_editor_tool(name: &str) -> bool {
    matches!(
        name,
        "inspect_editor_target" | "read_editor_draft" | "save_editor_draft"
    )
}

pub(super) fn needs_editor(name: &str) -> bool {
    is_editor_tool(name)
        || matches!(
            name,
            "edit_chapter_visual_plan_draft" | "save_chapter_visual_plan_draft"
        )
}
