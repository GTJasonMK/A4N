use std::io::Write;

use super::EpubParser;

#[test]
fn splits_chapters_from_headings_in_one_html_file() {
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

    let result = EpubParser
        .parse(&buffer.into_inner(), "章节拆分测试")
        .unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "Chapter 1 Start");
    assert_eq!(result.chapters[1].title, "Chapter 2 Continue");
}

#[test]
fn uses_opf_spine_order_instead_of_zip_entry_order() {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip_writer
            .start_file("OEBPS/chapter2.xhtml", options)
            .unwrap();
        zip_writer
            .write_all(br#"<html><head><title>Chapter 2</title></head><body><h1>Chapter 2</h1><p>Second.</p></body></html>"#)
            .unwrap();
        zip_writer.start_file("OEBPS/nav.xhtml", options).unwrap();
        zip_writer
            .write_all(
                br#"<html><head><title>Contents</title></head><body><nav><a href="chapter1.xhtml">Chapter 1</a><a href="chapter2.xhtml">Chapter 2</a></nav></body></html>"#,
            )
            .unwrap();
        zip_writer.start_file("OEBPS/content.opf", options).unwrap();
        zip_writer
            .write_all(
                br#"<package>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
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
            .write_all(br#"<html><head><title>Chapter 1</title></head><body><h1>Chapter 1</h1><p>First.</p></body></html>"#)
            .unwrap();
        zip_writer.finish().unwrap();
    }

    let result = EpubParser
        .parse(&buffer.into_inner(), "书脊排序测试")
        .unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "Chapter 1");
    assert_eq!(result.chapters[1].title, "Chapter 2");
}

#[test]
fn resolves_parent_relative_manifest_href() {
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

    let result = EpubParser
        .parse(&buffer.into_inner(), "相对路径测试")
        .unwrap();

    assert_eq!(result.chapters.len(), 2);
    assert_eq!(result.chapters[0].title, "Chapter 1");
    assert_eq!(result.chapters[1].title, "Chapter 2");
}
