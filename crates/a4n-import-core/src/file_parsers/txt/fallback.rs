use std::collections::HashMap;

use super::super::{ParseResult, ParsedChapter};
use super::trim_imported_text;

pub(super) fn whole_text_parse_result(text: &str) -> ParseResult {
    ParseResult {
        chapters: vec![ParsedChapter {
            number: 1,
            title: "完整内容".to_string(),
            content: trim_imported_text(text).to_string(),
            start_offset: 0,
            end_offset: text.len(),
        }],
        detected_encoding: "utf-8".to_string(),
        metadata: HashMap::new(),
        warnings: vec!["未检测到章节标记，整个文本作为一个章节处理。".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::whole_text_parse_result;

    #[test]
    fn builds_whole_text_single_chapter_fallback() {
        let text = "  没有章节标题的正文。\n";
        let result = whole_text_parse_result(text);

        assert_eq!(result.chapters.len(), 1);
        let chapter = &result.chapters[0];
        assert_eq!(chapter.number, 1);
        assert_eq!(chapter.title, "完整内容");
        assert_eq!(chapter.content, "没有章节标题的正文。");
        assert_eq!(chapter.start_offset, 0);
        assert_eq!(chapter.end_offset, text.len());
        assert_eq!(result.detected_encoding, "utf-8");
        assert!(result.metadata.is_empty());
        assert_eq!(
            result.warnings,
            ["未检测到章节标记，整个文本作为一个章节处理。"]
        );
    }
}
