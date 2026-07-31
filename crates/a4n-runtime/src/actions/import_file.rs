mod parsing;
mod upload;

use parsing::parse_import_file_bytes;
use upload::decode_import_upload;

pub use a4n_api_contract::import::{
    ImportFileResult, ImportUploadedFileInput, ParsedChapterSummary,
};

pub fn import_uploaded_file(input: ImportUploadedFileInput) -> Result<ImportFileResult, String> {
    let upload = decode_import_upload(input)?;
    parse_import_file_bytes(&upload.file_type, &upload.file_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use std::io::Write;

    #[test]
    fn parse_import_file_bytes_accepts_markdown_as_plain_text() {
        let markdown = "# 第1章 开始\n开篇正文。\n\n## 第2章 继续\n后续正文。";

        let result = parse_import_file_bytes("markdown", markdown.as_bytes())
            .expect("markdown should be parsed as text");

        assert_eq!(result.chapters.len(), 2);
        assert_eq!(result.chapters[0].number, 1);
        assert_eq!(result.chapters[0].title, "第1章 开始");
        assert_eq!(result.chapters[1].title, "第2章 继续");
        assert_eq!(result.detected_encoding, "utf-8");
        assert!(result.total_words > 0);
    }

    #[test]
    fn parse_import_file_bytes_accepts_md_alias_case_insensitively() {
        let markdown = "# Chapter 1 Start\nOpening.\n\n# Chapter 2 Continue\nNext.";

        let result =
            parse_import_file_bytes(" MD ", markdown.as_bytes()).expect("md alias should parse");

        assert_eq!(result.chapters.len(), 2);
        assert_eq!(result.chapters[0].title, "Chapter 1 Start");
        assert_eq!(result.chapters[1].title, "Chapter 2 Continue");
    }

    #[test]
    fn parse_import_file_bytes_rejects_unknown_file_type() {
        let err = parse_import_file_bytes("pdf", b"content")
            .expect_err("unsupported file type should fail");

        assert!(err.contains("不支持的文件类型: pdf"));
    }

    #[test]
    fn import_uploaded_file_decodes_base64_and_parses() {
        let body =
            base64::engine::general_purpose::STANDARD.encode("# 第1章 上传\n上传正文。".as_bytes());
        let result = import_uploaded_file(ImportUploadedFileInput {
            file_name: "novel.md".to_string(),
            file_type: "markdown".to_string(),
            data_base64: body,
        })
        .expect("uploaded import file should parse");

        assert_eq!(result.chapters.len(), 1);
        assert_eq!(result.chapters[0].title, "第1章 上传");
    }

    #[test]
    fn parse_import_file_bytes_does_not_duplicate_epub_headings() {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut archive = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            archive.start_file("OEBPS/content.opf", options).unwrap();
            archive
                .write_all(
                    br#"<package><manifest>
<item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/>
<item id="c2" href="c2.xhtml" media-type="application/xhtml+xml"/>
</manifest><spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#,
                )
                .unwrap();
            archive.start_file("OEBPS/c1.xhtml", options).unwrap();
            archive
                .write_all(
                    r#"<html><head><title>第1章 开始</title></head><body><h1>第1章 开始</h1><p>开篇正文。</p></body></html>"#
                        .as_bytes(),
                )
                .unwrap();
            archive.start_file("OEBPS/c2.xhtml", options).unwrap();
            archive
                .write_all(
                    r#"<html><head><title>第2章 继续</title></head><body><h1>第2章 继续</h1><p>后续正文。</p></body></html>"#
                        .as_bytes(),
                )
                .unwrap();
            archive.finish().unwrap();
        }

        let bytes = buffer.into_inner();
        let parsed = a4n_import_core::file_parsers::EpubParser
            .parse(&bytes)
            .unwrap();
        assert_eq!(
            parsed.chapters.len(),
            2,
            "direct parser titles: {:?}",
            parsed
                .chapters
                .iter()
                .map(|chapter| chapter.title.as_str())
                .collect::<Vec<_>>()
        );
        let result = parse_import_file_bytes("epub", &bytes).unwrap();

        assert_eq!(result.chapters.len(), 2);
        assert_eq!(result.chapters[0].title, "第1章 开始");
        assert_eq!(result.chapters[1].title, "第2章 继续");
    }
}
