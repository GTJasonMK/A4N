pub(super) fn looks_like_structured_or_fenced_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}
