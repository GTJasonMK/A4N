use a4n_services::import::dedup;
use a4n_services::import::file_parsers::{DocxParser, EpubParser, TxtParser};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

const MAX_UPLOADED_IMPORT_FILE_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct ImportUploadedFileInput {
    /// 用户选择的源文件名，仅用于错误信息和审核记录展示
    pub file_name: String,
    /// 文件类型：txt / md / markdown / epub / docx
    pub file_type: String,
    /// 文件内容 base64，可带 data URL 前缀
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedChapterSummary {
    /// 章节序号（从 1 开始）
    pub number: i32,
    /// 章节标题
    pub title: String,
    /// 章节正文
    pub content: String,
    /// SimHash 指纹（16 进制字符串）
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFileResult {
    /// 解析出的章节列表
    pub chapters: Vec<ParsedChapterSummary>,
    /// 检测到的文件编码
    pub detected_encoding: String,
    /// 总字数
    pub total_words: i32,
    /// 解析警告
    pub warnings: Vec<String>,
}

pub fn import_uploaded_file(input: ImportUploadedFileInput) -> Result<ImportFileResult, String> {
    let file_name = input.file_name.trim();
    if file_name.is_empty() {
        return Err("上传导入文件缺少文件名".to_string());
    }
    let raw = strip_data_url_prefix(&input.data_base64).trim();
    if raw.is_empty() {
        return Err("上传导入文件数据不能为空".to_string());
    }
    let file_bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("上传导入文件 base64 解码失败：{err}"))?;
    if file_bytes.is_empty() {
        return Err("上传导入文件不能为空".to_string());
    }
    if file_bytes.len() > MAX_UPLOADED_IMPORT_FILE_BYTES {
        return Err(format!(
            "上传导入文件超过 {}MB",
            MAX_UPLOADED_IMPORT_FILE_BYTES / 1024 / 1024
        ));
    }
    parse_import_file_bytes(&input.file_type, &file_bytes)
}

fn parse_import_file_bytes(file_type: &str, file_bytes: &[u8]) -> Result<ImportFileResult, String> {
    let text_content = import_file_text_content(file_type, file_bytes)?;

    let parser = TxtParser::new();
    let parse_result = parser
        .parse(&text_content)
        .map_err(|e| format!("章节解析失败: {e}"))?;

    let chapters = parse_result
        .chapters
        .iter()
        .map(|ch| {
            let hash = dedup::compute_simhash(&ch.content, 3);
            ParsedChapterSummary {
                number: ch.number,
                title: ch.title.clone(),
                content: ch.content.clone(),
                content_hash: Some(format!("{:016x}", hash.0)),
            }
        })
        .collect::<Vec<_>>();

    let total_words = chapters
        .iter()
        .map(|chapter| chapter.content.chars().count() as i32)
        .sum();

    Ok(ImportFileResult {
        chapters,
        detected_encoding: parse_result.detected_encoding,
        total_words,
        warnings: parse_result.warnings,
    })
}

fn import_file_text_content(file_type: &str, file_bytes: &[u8]) -> Result<String, String> {
    match file_type.trim().to_ascii_lowercase().as_str() {
        "txt" | "md" | "markdown" => {
            String::from_utf8(file_bytes.to_vec()).map_err(|e| format!("无法解析文本编码: {e}"))
        }
        "epub" => {
            let result = EpubParser::new()
                .parse(file_bytes)
                .map_err(|e| format!("EPUB 解析失败: {e}"))?;
            Ok(result
                .chapters
                .iter()
                .map(|chapter| format!("{}\n{}\n", chapter.title, chapter.content))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "docx" => {
            let result = DocxParser::new()
                .parse(file_bytes)
                .map_err(|e| format!("DOCX 解析失败: {e}"))?;
            Ok(result
                .chapters
                .iter()
                .map(|chapter| format!("{}\n{}\n", chapter.title, chapter.content))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        other => Err(format!("不支持的文件类型: {other}")),
    }
}

fn strip_data_url_prefix(value: &str) -> &str {
    value
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
