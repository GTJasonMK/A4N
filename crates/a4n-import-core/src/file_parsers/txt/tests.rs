use super::TxtParser;

#[test]
fn splits_chapters() {
    let text = "第1章 开始\n这是第一章的内容。\n\n第2章 旅程\n这是第二章的内容。";
    let result = TxtParser::new().parse(text).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].number, 1);
    assert!(result.chapters[0].content.contains("第一章的内容"));
    assert_eq!(result.chapters[1].number, 2);
}

#[test]
fn handles_utf8_bom_before_first_chapter() {
    let text = "\u{feff}第1章 开始\n开篇正文。\n\n第2章 继续\n后续正文。";
    let result = TxtParser::new().parse(text).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "第1章 开始");
    assert!(result.chapters[0].content.starts_with("第1章 开始"));
    assert_eq!(result.chapters[1].title, "第2章 继续");
}

#[test]
fn falls_back_to_single_chapter_without_markers() {
    let text = "这是一段没有任何章节标记的文本。";
    let result = TxtParser::new().parse(text).unwrap();

    assert_eq!(result.chapters.len(), 1);
    assert!(!result.warnings.is_empty());
}

#[test]
fn splits_markdown_chapter_headings_as_plain_text() {
    let text = "# 第1章 开始\n开篇正文。\n\n## Chapter 2 Continue\nSecond part.";
    let result = TxtParser::new().parse(text).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "第1章 开始");
    assert_eq!(result.chapters[1].title, "Chapter 2 Continue");
    assert!(result.chapters[0].content.starts_with("# 第1章 开始"));
}
