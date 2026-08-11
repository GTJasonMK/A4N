mod archive;
mod document;
#[cfg(test)]
mod tests;

use crate::ImportTransformError;

use super::ParseResult;
use super::shared::parse_extracted_document;
use archive::read_docx_archive;

/// DOCX 文件解析器
///
/// 使用 zip + XML 解析提取文本内容。
pub struct DocxParser;

impl DocxParser {
    /// 从 DOCX 字节数据解析文本内容
    pub fn parse(
        &self,
        data: &[u8],
        source_title: &str,
    ) -> Result<ParseResult, ImportTransformError> {
        let archive = read_docx_archive(data)?;
        parse_extracted_document(
            &archive.text_content,
            archive.title.as_deref(),
            source_title,
        )
    }
}
