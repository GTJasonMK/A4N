use super::EpubHtmlEntry;

pub(in crate::file_parsers) fn order_epub_html_entries(
    entries: Vec<EpubHtmlEntry>,
    spine_paths: &[String],
) -> Vec<String> {
    if spine_paths.is_empty() {
        return entries.into_iter().map(|entry| entry.text).collect();
    }
    let mut used = vec![false; entries.len()];
    let mut ordered = Vec::with_capacity(entries.len());
    for spine_path in spine_paths {
        if let Some((index, entry)) = entries
            .iter()
            .enumerate()
            .find(|(index, entry)| !used[*index] && entry.path == *spine_path)
        {
            used[index] = true;
            ordered.push(entry.text.clone());
        }
    }
    if ordered.is_empty() {
        return entries.into_iter().map(|entry| entry.text).collect();
    }
    ordered
}
