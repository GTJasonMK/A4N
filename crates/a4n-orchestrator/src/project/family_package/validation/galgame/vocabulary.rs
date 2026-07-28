use a4n_core::models::galgame::galgame_line;

pub(super) fn normalized_galgame_expression(line: &galgame_line::Model) -> Option<&str> {
    line.expression
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::project::family_package::validation) fn normalized_galgame_position(
    line: &galgame_line::Model,
) -> Option<&str> {
    line.position
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::project::family_package::validation) fn is_valid_galgame_expression(
    value: &str,
) -> bool {
    matches!(
        value,
        "neutral" | "smile" | "sad" | "angry" | "surprised" | "shy" | "serious"
    )
}

pub(in crate::project::family_package::validation) fn is_valid_galgame_position(
    value: &str,
) -> bool {
    matches!(value, "left" | "center" | "right")
}
