use serde::{Deserialize, Serialize};

mod parsing;
mod upload;

use parsing::parse_import_file_bytes;
use upload::decode_import_upload;

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
    let upload = decode_import_upload(input)?;
    parse_import_file_bytes(&upload.file_type, &upload.file_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

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
