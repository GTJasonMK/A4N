use regex::Regex;

use crate::ImportTransformError;

use super::helpers::trim_imported_text;
use crate::file_parsers::ParsedChapter;

mod matches;

use matches::find_chapter_matches;

pub(super) fn split_into_chapters(
    patterns: &[Regex],
    text: &str,
) -> Result<Vec<ParsedChapter>, ImportTransformError> {
    let matches = find_chapter_matches(patterns, text);
    if matches.is_empty() {
        return Ok(vec![]);
    }

    let mut chapters = Vec::with_capacity(matches.len());
    for i in 0..matches.len() {
        let (start, title) = matches[i];
        let end = matches.get(i + 1).map_or(text.len(), |next| next.0);
        let content = trim_imported_text(&text[start..end]).to_string();
        let number = i32::try_from(i + 1).map_err(|_| {
            ImportTransformError::InvalidDocument("TXT 解析章节数量超出 i32 范围".to_string())
        })?;
        chapters.push(ParsedChapter {
            number,
            title: title.to_string(),
            content,
        });
    }
    Ok(chapters)
}
