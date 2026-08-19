mod chapters;
mod helpers;
#[cfg(test)]
mod tests;

use crate::ImportTransformError;
use regex::Regex;

use super::{ParseResult, ParsedChapter};
use helpers::trim_imported_text;

/// TXT 文件解析器
///
/// 按章节标记（如"第X章"、"Chapter X"、"第X回"等模式）分割文本。
pub struct TxtParser {
    /// 章节标题正则模式
    chapter_patterns: Vec<Regex>,
}

#[allow(clippy::new_without_default)]
impl TxtParser {
    pub fn new() -> Self {
        Self {
            chapter_patterns: vec![
                Regex::new(r"(?m)^\u{feff}?(?:#{1,6}\s*)?第[一二三四五六七八九十百千\d]+[章节回]")
                    .unwrap(),
                Regex::new(r"(?m)^\u{feff}?(?:#{1,6}\s*)?Chapter\s+\d+").unwrap(),
                Regex::new(r"(?m)^\u{feff}?(?:#{1,6}\s*)?第[一二三四五六七八九十百千\d]+卷")
                    .unwrap(),
            ],
        }
    }

    /// 解析 TXT 文本，按章节分割
    pub fn parse(
        &self,
        text: &str,
        document_title: &str,
    ) -> Result<ParseResult, ImportTransformError> {
        let mut chapters = chapters::split_into_chapters(&self.chapter_patterns, text)?;
        if chapters.is_empty() {
            let title = document_title.trim();
            if title.is_empty() {
                return Err(ImportTransformError::InvalidDocument(
                    "TXT 未检测到章节标记且缺少文档标题".to_string(),
                ));
            }
            chapters.push(ParsedChapter {
                number: 1,
                title: title.to_string(),
                content: trim_imported_text(text).to_string(),
            });
        }

        Ok(ParseResult { chapters })
    }
}
