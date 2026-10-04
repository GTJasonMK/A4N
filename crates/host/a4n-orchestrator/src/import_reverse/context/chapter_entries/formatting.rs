pub(super) fn text_preview(content: &str, max_chars: usize) -> String {
    let mut text = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() > max_chars {
        text = text.chars().take(max_chars).collect::<String>();
        text.push('…');
    }
    text
}

pub(super) fn chapter_summary_text(ch: &ChapterRecord) -> Option<String> {
    ch.analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| {
                    !summary.is_empty()
                        && !summary.starts_with(&format!("第{}章：", ch.chapter_number))
                })
                .map(str::to_string)
        })
}

fn chapter_title(title: Option<&str>) -> Option<String> {
    title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn add_optional_chapter_title(entry: &mut Map<String, Value>, ch: &ChapterRecord) {
    if let Some(title) = chapter_title(ch.title.as_deref()) {
        entry.insert("title".to_string(), json!(title));
    }
}

pub(super) fn add_summary_or_preview(
    entry: &mut Map<String, Value>,
    ch: &ChapterRecord,
    max_chars: usize,
) {
    if let Some(summary) = chapter_summary_text(ch) {
        entry.insert("summary".to_string(), json!(summary));
    } else {
        entry.insert(
            "content_preview".to_string(),
            json!(text_preview(&ch.content, max_chars)),
        );
    }
}
use a4n_domain::chapter::ChapterRecord;
use serde_json::{Map, Value, json};

use a4n_chapter_analysis::summary_text;
