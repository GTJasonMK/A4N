use super::{
    DocxParser, EpubParser, TxtParser,
    epub::{resolve_epub_href, strip_html_tags},
    shared::extract_xml_text,
    txt::{line_end_at_or_after, line_start_at_or_before},
};

#[test]
fn test_txt_parser_with_chapters() {
    let text = "第1章 开始\n这是第一章的内容。\n\n第2章 旅程\n这是第二章的内容。";
    let parser = TxtParser::new();
    let result = parser.parse(text).unwrap();
    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].number, 1);
    assert!(result.chapters[0].content.contains("第一章的内容"));
    assert_eq!(result.chapters[1].number, 2);
}

#[test]
fn txt_parser_handles_utf8_bom_before_first_chapter() {
    let text = "\u{feff}第1章 开始\n开篇正文。\n\n第2章 继续\n后续正文。";
    let parser = TxtParser::new();
    let result = parser.parse(text).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "第1章 开始");
    assert!(result.chapters[0].content.starts_with("第1章 开始"));
    assert_eq!(result.chapters[1].title, "第2章 继续");
}

#[test]
fn test_txt_parser_no_chapters() {
    let text = "这是一段没有任何章节标记的文本。";
    let parser = TxtParser::new();
    let result = parser.parse(text).unwrap();
    assert_eq!(result.chapters.len(), 1);
    assert!(!result.warnings.is_empty());
}

#[test]
fn txt_parser_splits_markdown_chapter_headings_as_plain_text() {
    let text = "# 第1章 开始\n开篇正文。\n\n## Chapter 2 Continue\nSecond part.";
    let parser = TxtParser::new();
    let result = parser.parse(text).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "第1章 开始");
    assert_eq!(result.chapters[1].title, "Chapter 2 Continue");
    assert!(result.chapters[0].content.starts_with("# 第1章 开始"));
}

#[test]
fn line_bounds_cover_first_middle_and_last_lines() {
    let text = "第1章 开始\n正文\n第2章 结束";

    assert_eq!(line_start_at_or_before(text, 0), 0);
    assert_eq!(line_end_at_or_after(text, 0), "第1章 开始".len());

    let middle = text.find("正文").expect("middle line exists");
    assert_eq!(line_start_at_or_before(text, middle), "第1章 开始\n".len());
    assert_eq!(line_end_at_or_after(text, middle), "第1章 开始\n正文".len());

    let last = text.find("第2章").expect("last line exists");
    assert_eq!(
        line_start_at_or_before(text, last),
        "第1章 开始\n正文\n".len()
    );
    assert_eq!(line_end_at_or_after(text, last), text.len());
}

#[test]
fn test_strip_html_tags() {
    let html = "<p>Hello <b>World</b></p>";
    assert_eq!(strip_html_tags(html), "Hello World");
}

#[test]
fn strip_html_tags_decodes_entities_after_removing_markup() {
    let html = "<p>A &lt; B &amp;&amp; C&nbsp;&#33; &#x4e2d;</p>";
    assert_eq!(strip_html_tags(html), "A < B && C ! 中");
}

#[test]
fn strip_html_tags_preserves_block_boundaries_for_chapter_detection() {
    let html = "<h1>第1章 开始</h1><p>开篇正文。</p><h1>第2章 继续</h1><p>后续正文。</p>";
    let text = strip_html_tags(html);

    let result = TxtParser::new().parse(&text).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "第1章 开始");
    assert_eq!(result.chapters[1].title, "第2章 继续");
}

#[test]
fn epub_parser_splits_chapters_from_headings_in_one_html_file() {
    use std::io::Write;

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut buffer);
        zip_writer
            .start_file(
                "OEBPS/chapter.xhtml",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        zip_writer
            .write_all(
                br#"<html><body><h1>Chapter 1 Start</h1><p>Opening.</p><h1>Chapter 2 Continue</h1><p>Next.</p></body></html>"#,
            )
            .unwrap();
        zip_writer.finish().unwrap();
    }

    let result = EpubParser::new().parse(&buffer.into_inner()).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "Chapter 1 Start");
    assert_eq!(result.chapters[1].title, "Chapter 2 Continue");
}

#[test]
fn epub_parser_uses_opf_spine_order_instead_of_zip_entry_order() {
    use std::io::Write;

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip_writer
            .start_file("OEBPS/chapter2.xhtml", options)
            .unwrap();
        zip_writer
            .write_all(br#"<html><body><h1>Chapter 2</h1><p>Second.</p></body></html>"#)
            .unwrap();

        zip_writer.start_file("OEBPS/content.opf", options).unwrap();
        zip_writer
            .write_all(
                br#"<package>
  <manifest>
    <item id="c1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
    <itemref idref="c2"/>
  </spine>
</package>"#,
            )
            .unwrap();

        zip_writer
            .start_file("OEBPS/chapter1.xhtml", options)
            .unwrap();
        zip_writer
            .write_all(br#"<html><body><h1>Chapter 1</h1><p>First.</p></body></html>"#)
            .unwrap();

        zip_writer.finish().unwrap();
    }

    let result = EpubParser::new().parse(&buffer.into_inner()).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "Chapter 1");
    assert_eq!(result.chapters[1].title, "Chapter 2");
}

#[test]
fn epub_spine_resolves_parent_relative_manifest_href() {
    use std::io::Write;

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip_writer
            .start_file("OEBPS/chapter2.xhtml", options)
            .unwrap();
        zip_writer
            .write_all(br#"<html><body><h1>Chapter 2</h1><p>Second.</p></body></html>"#)
            .unwrap();

        zip_writer.start_file("OEBPS/content.opf", options).unwrap();
        zip_writer
            .write_all(
                br#"<package>
  <manifest>
    <item id="c1" href="../chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
    <itemref idref="c2"/>
  </spine>
</package>"#,
            )
            .unwrap();

        zip_writer.start_file("chapter1.xhtml", options).unwrap();
        zip_writer
            .write_all(br#"<html><body><h1>Chapter 1</h1><p>First.</p></body></html>"#)
            .unwrap();

        zip_writer.finish().unwrap();
    }

    let result = EpubParser::new().parse(&buffer.into_inner()).unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "Chapter 1");
    assert_eq!(result.chapters[1].title, "Chapter 2");
}

#[test]
fn resolve_epub_href_normalizes_dot_segments() {
    assert_eq!(
        resolve_epub_href("OEBPS/package/content.opf", "../Text/./Ch1.XHTML#section"),
        "oebps/text/ch1.xhtml"
    );
}

#[test]
fn extract_xml_text_decodes_xml_entities() {
    let xml = "<metadata><dc:title>A4N &amp; Co</dc:title></metadata>";
    assert_eq!(extract_xml_text(xml, "title"), Some("A4N & Co".to_string()));
}

#[test]
fn docx_parser_decodes_text_node_entities() {
    use std::io::Write;

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut buffer);
        zip_writer
            .start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        zip_writer
            .write_all(
                br#"<w:document><w:body><w:p><w:r><w:t>A4N &amp; Co</w:t></w:r></w:p></w:body></w:document>"#,
            )
            .unwrap();
        zip_writer.finish().unwrap();
    }

    let result = DocxParser::new().parse(&buffer.into_inner()).unwrap();

    assert_eq!(result.chapters[0].content, "A4N & Co");
}

#[test]
fn docx_parser_reads_pretty_printed_paragraph_xml() {
    use std::io::Write;

    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut buffer);
        zip_writer
            .start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        zip_writer
            .write_all(
                br#"<w:document>
  <w:body>
    <w:p>
      <w:r><w:t>A4N</w:t></w:r>
      <w:r><w:t> body</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#,
            )
            .unwrap();
        zip_writer.finish().unwrap();
    }

    let result = DocxParser::new().parse(&buffer.into_inner()).unwrap();

    assert_eq!(result.chapters[0].content, "A4N body");
}
