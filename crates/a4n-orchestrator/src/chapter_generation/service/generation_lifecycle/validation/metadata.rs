use super::super::super::response_json::GeneratedChapterMetadata;
use super::super::session::ChapterGenerationSession;

mod fallback;
mod outline;

use fallback::summary_from_generated_content;
use outline::extract_marked_outline_value;

pub(super) fn generated_chapter_metadata(
    session: &ChapterGenerationSession,
    chapter_outline: &str,
    raw_content: &str,
) -> GeneratedChapterMetadata {
    let outline_title =
        extract_marked_outline_value(chapter_outline, &["章节标题", "标题", "title"]);
    let outline_summary = extract_marked_outline_value(
        chapter_outline,
        &["章节摘要", "摘要", "summary", "大纲摘要"],
    );
    let title = session
        .existing_chapter
        .as_ref()
        .and_then(|chapter| chapter.title.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or(outline_title)
        .unwrap_or_else(|| format!("第{}章", session.chapter_number));
    let summary = session
        .existing_chapter
        .as_ref()
        .and_then(|chapter| chapter.summary.as_deref())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
        .or(outline_summary)
        .unwrap_or_else(|| summary_from_generated_content(raw_content));
    GeneratedChapterMetadata { title, summary }
}
