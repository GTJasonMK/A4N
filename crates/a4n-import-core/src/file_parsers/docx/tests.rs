use std::io::Write;

use super::DocxParser;

#[test]
fn decodes_text_node_entities() {
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

    let result = DocxParser.parse(&buffer.into_inner()).unwrap();

    assert_eq!(result.chapters[0].content, "A4N & Co");
}

#[test]
fn reads_pretty_printed_paragraph_xml() {
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

    let result = DocxParser.parse(&buffer.into_inner()).unwrap();

    assert_eq!(result.chapters[0].content, "A4N body");
}
