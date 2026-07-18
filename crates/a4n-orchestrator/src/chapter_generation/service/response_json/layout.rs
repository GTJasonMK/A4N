pub fn normalize_generated_chapter_content_layout(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let candidates: Vec<String> = if normalized.contains('\n') {
        normalized
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            Vec::new()
        } else {
            vec![trimmed.to_string()]
        }
    };

    candidates
        .iter()
        .flat_map(|paragraph| split_long_paragraph(paragraph))
        .collect::<Vec<_>>()
        .join("\n\n")
}

mod paragraphs;

#[cfg(test)]
mod tests;

use paragraphs::split_long_paragraph;
