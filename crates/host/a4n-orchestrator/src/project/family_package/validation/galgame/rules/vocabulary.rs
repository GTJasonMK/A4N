pub(in crate::project::family_package::validation) fn normalized_galgame_position(
    line: &a4n_domain::project::ProjectFamilyGalgameLineRecord,
) -> Option<&str> {
    line.position
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(in crate::project::family_package::validation) fn is_valid_galgame_expression(
    value: &str,
) -> bool {
    a4n_domain::galgame::GALGAME_EXPRESSIONS.contains(&value)
}

pub(in crate::project::family_package::validation) fn is_valid_galgame_position(
    value: &str,
) -> bool {
    a4n_domain::galgame::GalgameStagePositionV2::parse(value).is_some()
}
