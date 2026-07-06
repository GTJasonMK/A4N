use a4n_core::error::AppError;
use regex::Regex;
use zip::result::ZipError;

use super::shared::{decode_xml_entities, extract_xml_text};
use super::{ParseResult, TxtParser};

/// DOCX 文件解析器
///
/// 使用 zip + XML 解析提取文本内容。
pub struct DocxParser;

impl DocxParser {
    pub fn new() -> Self {
        Self
    }

    /// 从 DOCX 字节数据解析文本内容
    pub fn parse(&self, data: &[u8]) -> Result<ParseResult, AppError> {
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| AppError::InvalidArgument(format!("无法打开 DOCX 文件: {e}")))?;

        let mut paragraphs = Vec::new();

        let mut doc_file = archive.by_name("word/document.xml").map_err(|e| {
            AppError::InvalidArgument(format!("DOCX 缺少 word/document.xml 或无法读取: {e}"))
        })?;
        let mut xml_content = String::new();
        std::io::Read::read_to_string(&mut doc_file, &mut xml_content)
            .map_err(|e| AppError::InvalidArgument(format!("无法读取 DOCX 内容: {e}")))?;
        drop(doc_file);

        let text_re = Regex::new(r"<w:t[^>]*>([^<]*)</w:t>").unwrap();
        let para_re = Regex::new(r"(?s)<w:p(?:\s|>).*?</w:p>").unwrap();
        for para_match in para_re.find_iter(&xml_content) {
            let para_xml = para_match.as_str();
            let mut para_text = String::new();
            for cap in text_re.captures_iter(para_xml) {
                para_text.push_str(&decode_xml_entities(&cap[1]));
            }
            if !para_text.trim().is_empty() {
                paragraphs.push(para_text);
            }
        }

        let text_content = paragraphs.join("\n\n");
        let mut metadata = std::collections::HashMap::new();

        match archive.by_name("docProps/core.xml") {
            Ok(mut meta_file) => {
                let mut xml = String::new();
                std::io::Read::read_to_string(&mut meta_file, &mut xml)
                    .map_err(|e| AppError::InvalidArgument(format!("无法读取 DOCX 元数据: {e}")))?;
                if let Some(title) = extract_xml_text(&xml, "title") {
                    metadata.insert("title".to_string(), title);
                }
                if let Some(creator) = extract_xml_text(&xml, "creator") {
                    metadata.insert("author".to_string(), creator);
                }
            }
            Err(ZipError::FileNotFound) => {}
            Err(e) => {
                return Err(AppError::InvalidArgument(format!(
                    "无法读取 DOCX 元数据条目: {e}"
                )));
            }
        }

        let txt_parser = TxtParser::new();
        let mut result = txt_parser.parse(&text_content)?;
        result.detected_encoding = "utf-8".to_string();
        result.metadata = metadata;

        Ok(result)
    }
}
