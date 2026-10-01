use a4n_domain::chapter::ChapterRecord;

pub fn chapter_display_title(ch: &ChapterRecord) -> String {
    ch.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("第{}章", ch.chapter_number))
}
