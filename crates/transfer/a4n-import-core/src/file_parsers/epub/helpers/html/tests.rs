use super::strip_html_tags;
use crate::file_parsers::TxtParser;

#[test]
fn removes_markup() {
    let html = "<p>Hello <b>World</b></p>";
    assert_eq!(strip_html_tags(html), "Hello World");
}

#[test]
fn decodes_entities_after_removing_markup() {
    let html = "<p>A &lt; B &amp;&amp; C&nbsp;&#33; &#x4e2d;</p>";
    assert_eq!(strip_html_tags(html), "A < B && C ! 中");
}

#[test]
fn preserves_block_boundaries_for_chapter_detection() {
    let html = "<h1>第1章 开始</h1><p>开篇正文。</p><h1>第2章 继续</h1><p>后续正文。</p>";
    let text = strip_html_tags(html);

    let result = TxtParser::new().parse(&text, "HTML 文本测试").unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "第1章 开始");
    assert_eq!(result.chapters[1].title, "第2章 继续");
}

#[test]
fn ignores_document_head_when_body_is_present() {
    let html = "<html><head><title>第1章 重复标题</title></head><body><h1>第1章 正文标题</h1><p>正文。</p></body></html>";

    assert_eq!(strip_html_tags(html), "第1章 正文标题\n正文。");
}
