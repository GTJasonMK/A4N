use a4n_error::AppError;

use super::super::super::response_json::GeneratedChapterMetadata;
use super::super::session::ChapterGenerationSession;

mod outline;

use outline::extract_marked_outline_value;

pub(super) fn generated_chapter_metadata(
    session: &ChapterGenerationSession,
    chapter_outline: &str,
    raw_content: &str,
) -> Result<GeneratedChapterMetadata, AppError> {
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
        .or_else(|| summary_from_generated_content(raw_content))
        .ok_or_else(|| {
            AppError::InvalidArgument("LLM 未返回可用于摘要的章节正文，未写入正文".to_string())
        })?;
    Ok(GeneratedChapterMetadata { title, summary })
}

fn summary_from_generated_content(raw_content: &str) -> Option<String> {
    let mut summary = raw_content
        .trim()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?
        .chars()
        .take(120)
        .collect::<String>();
    if summary.chars().count() == 120 {
        summary.push('…');
    }
    Some(summary)
}
