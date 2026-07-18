#[cfg(test)]
mod tests;

pub fn trim_imported_text(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

pub(super) fn normalize_chapter_title_line(line: &str) -> &str {
    let title = trim_imported_text(line);
    let hash_count = title.bytes().take_while(|byte| *byte == b'#').count();
    if (1..=6).contains(&hash_count) {
        return trim_imported_text(&title[hash_count..]);
    }
    title
}

pub fn line_start_at_or_before(text: &str, offset: usize) -> usize {
    match text[..offset].rfind('\n') {
        Some(position) => position + 1,
        None => 0,
    }
}

pub fn line_end_at_or_after(text: &str, offset: usize) -> usize {
    match text[offset..].find('\n') {
        Some(position) => offset + position,
        None => text.len(),
    }
}
