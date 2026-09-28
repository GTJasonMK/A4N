use crate::file_parsers::{DocxParser, EpubParser, ParsedChapter, TxtParser};
use a4n_import_contract::{
    ImportParseDocumentInput, ImportParseDocumentOutput, MAX_PROJECT_TEXT_UPLOAD_BYTES,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub fn parse_document(
    input: ImportParseDocumentInput,
) -> Result<ImportParseDocumentOutput, crate::ImportTransformError> {
    if input.document_title.trim().is_empty() {
        return Err(crate::ImportTransformError::InvalidDocument(
            "上传导入文件缺少文件名".into(),
        ));
    }
    let raw = input.bytes_base64.trim();
    if raw.len() > MAX_PROJECT_TEXT_UPLOAD_BYTES.div_ceil(3) * 4 {
        return Err(crate::ImportTransformError::InvalidDocument(
            "上传导入文件超过 100MB".into(),
        ));
    }
    let bytes = BASE64
        .decode(raw)
        .map_err(|source| crate::ImportTransformError::Base64Decode {
            label: "上传导入文件".into(),
            source,
        })?;
    if bytes.is_empty() || bytes.len() > MAX_PROJECT_TEXT_UPLOAD_BYTES {
        return Err(crate::ImportTransformError::InvalidDocument(
            "上传导入文件不能为空或超过 100MB".into(),
        ));
    }
    Ok(ImportParseDocumentOutput {
        chapters: parse_import_file_bytes(&input.file_type, &bytes, &input.document_title)
            .map_err(crate::ImportTransformError::InvalidDocument)?,
    })
}

pub fn parse_import_file_bytes(
    file_type: &str,
    file_bytes: &[u8],
    document_title: &str,
) -> Result<Vec<ParsedChapter>, String> {
    let parse_result = match file_type.trim().to_ascii_lowercase().as_str() {
        "txt" | "markdown" => {
            let text = std::str::from_utf8(file_bytes)
                .map_err(|error| format!("无法解析文本编码: {error}"))?;
            TxtParser::new()
                .parse(text, document_title)
                .map_err(|error| format!("章节解析失败: {error}"))?
        }
        "epub" => EpubParser
            .parse(file_bytes, document_title)
            .map_err(|error| format!("EPUB 解析失败: {error}"))?,
        "docx" => DocxParser
            .parse(file_bytes, document_title)
            .map_err(|error| format!("DOCX 解析失败: {error}"))?,
        other => return Err(format!("不支持的文件类型: {other}")),
    };
    Ok(parse_result.chapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn document_wire_rejects_empty_and_invalid_base64() {
        for bytes_base64 in ["", "not-base64"] {
            assert!(
                parse_document(ImportParseDocumentInput {
                    file_type: "txt".into(),
                    bytes_base64: bytes_base64.into(),
                    document_title: "作品".into(),
                })
                .is_err()
            );
        }
        assert!(
            parse_document(ImportParseDocumentInput {
                file_type: "txt".into(),
                bytes_base64: BASE64.encode("正文"),
                document_title: " ".into(),
            })
            .is_err()
        );
    }

    #[test]
    fn parse_import_file_bytes_accepts_markdown_as_plain_text() {
        let markdown = "# 第1章 开始\n开篇正文。\n\n## 第2章 继续\n后续正文。";

        let result = parse_import_file_bytes("markdown", markdown.as_bytes(), "测试文档")
            .expect("markdown should be parsed as text");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].number, 1);
        assert_eq!(result[0].title, "第1章 开始");
        assert_eq!(result[1].title, "第2章 继续");
    }

    #[test]
    fn parse_import_file_bytes_rejects_unknown_file_type() {
        let err = parse_import_file_bytes("pdf", b"content", "测试文档")
            .expect_err("unsupported file type should fail");

        assert!(err.contains("不支持的文件类型: pdf"));
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
        let parsed = crate::file_parsers::EpubParser
            .parse(&bytes, "测试文档")
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
        let result = parse_import_file_bytes("epub", &bytes, "测试文档").unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "第1章 开始");
        assert_eq!(result[1].title, "第2章 继续");
    }
}
