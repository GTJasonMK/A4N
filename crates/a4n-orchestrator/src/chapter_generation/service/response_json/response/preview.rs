pub(super) fn response_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let prefix = trimmed.chars().take(180).collect::<String>();
    if trimmed.chars().count() <= 260 {
        return prefix;
    }
    let suffix = trimmed
        .chars()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix} … {suffix}")
}
