use crate::ImportTransformError;

use super::ParseResult;
use super::shared::parse_extracted_document;

mod archive;
mod helpers;
#[cfg(test)]
mod tests;
use archive::read_epub_archive;

/// EPUB 文件解析器
///
/// 使用 zip crate 解包 EPUB 容器，提取 XHTML 文本内容。
pub struct EpubParser;

impl EpubParser {
    pub fn new() -> Self {
        Self
    }

    /// 从 EPUB 字节数据解析文本内容
    pub fn parse(&self, data: &[u8]) -> Result<ParseResult, ImportTransformError> {
        let archive = read_epub_archive(data)?;
        parse_extracted_document(&archive.text_content, archive.metadata, archive.warnings)
    }
}
