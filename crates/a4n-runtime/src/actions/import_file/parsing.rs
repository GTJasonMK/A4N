use a4n_import_core::dedup;
use a4n_import_core::file_parsers::{DocxParser, EpubParser, TxtParser};

use super::{ImportFileResult, ParsedChapterSummary};

pub(super) fn parse_import_file_bytes(
    file_type: &str,
    file_bytes: &[u8],
) -> Result<ImportFileResult, String> {
    let text_content = import_file_text_content(file_type, file_bytes)?;

    let parse_result = TxtParser::new()
        .parse(&text_content)
        .map_err(|error| format!("章节解析失败: {error}"))?;
    let chapters = parse_result
        .chapters
        .iter()
        .map(|chapter| {
            let hash = dedup::compute_simhash(&chapter.content, 3);
            ParsedChapterSummary {
                number: chapter.number,
                title: chapter.title.clone(),
                content: chapter.content.clone(),
                content_hash: Some(format!("{:016x}", hash.0)),
            }
        })
        .collect::<Vec<_>>();
    let total_words = chapters.iter().try_fold(0_i32, |total, chapter| {
        let chapter_words = i32::try_from(chapter.content.chars().count())
            .map_err(|_| format!("第 {} 章正文字符数超出 i32 范围", chapter.number))?;
        total
            .checked_add(chapter_words)
            .ok_or_else(|| "导入文件正文总字符数超出 i32 范围".to_string())
    })?;

    Ok(ImportFileResult {
        chapters,
        detected_encoding: parse_result.detected_encoding,
        total_words,
        warnings: parse_result.warnings,
    })
}

fn import_file_text_content(file_type: &str, file_bytes: &[u8]) -> Result<String, String> {
    match file_type.trim().to_ascii_lowercase().as_str() {
        "txt" | "md" | "markdown" => String::from_utf8(file_bytes.to_vec())
            .map_err(|error| format!("无法解析文本编码: {error}")),
        "epub" => parsed_document_text(
            EpubParser::new()
                .parse(file_bytes)
                .map_err(|error| format!("EPUB 解析失败: {error}"))?,
        ),
        "docx" => parsed_document_text(
            DocxParser::new()
                .parse(file_bytes)
                .map_err(|error| format!("DOCX 解析失败: {error}"))?,
        ),
        other => Err(format!("不支持的文件类型: {other}")),
    }
}

fn parsed_document_text(
    result: a4n_import_core::file_parsers::ParseResult,
) -> Result<String, String> {
    Ok(result
        .chapters
        .iter()
        .map(|chapter| format!("{}\n{}\n", chapter.title, chapter.content))
        .collect::<Vec<_>>()
        .join("\n"))
}
