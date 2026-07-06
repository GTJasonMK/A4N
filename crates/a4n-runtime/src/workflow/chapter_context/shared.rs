pub(super) fn chapter_context_label(chapter_number: i32, title: Option<&str>) -> String {
    match title.and_then(non_empty_trimmed) {
        Some(title) => format!("第{chapter_number}章《{title}》"),
        None => format!("第{chapter_number}章"),
    }
}

pub(super) fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut iter = text.trim().chars();
    let truncated = iter.by_ref().take(max_chars).collect::<String>();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub(super) fn non_empty_trimmed(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}
