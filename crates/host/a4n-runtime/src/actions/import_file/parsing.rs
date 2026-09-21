use a4n_import_core::file_parsers::{DocxParser, EpubParser, ParsedChapter, TxtParser};

pub(super) fn parse_import_file_bytes(
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
