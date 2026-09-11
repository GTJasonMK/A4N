use a4n_db_models::models::chapter::chapter;

pub fn chapter_display_title(ch: &chapter::Model) -> String {
    ch.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("第{}章", ch.chapter_number))
}
