mod chapters;
mod fallback;
mod helpers;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::ImportTransformError;
use regex::Regex;

use super::ParseResult;
pub(super) use helpers::trim_imported_text;

/// TXT 文件解析器
///
/// 按章节标记（如"第X章"、"Chapter X"、"第X回"等模式）分割文本。
pub struct TxtParser {
    /// 章节标题正则模式
    chapter_patterns: Vec<Regex>,
}

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
    pub fn parse(&self, text: &str) -> Result<ParseResult, ImportTransformError> {
        let chapters = chapters::split_into_chapters(&self.chapter_patterns, text)?;

        if chapters.is_empty() {
            return Ok(fallback::whole_text_parse_result(text));
        }

        Ok(ParseResult {
            chapters,
            detected_encoding: "utf-8".to_string(),
            metadata: HashMap::new(),
            warnings: Vec::new(),
        })
    }
}

impl Default for TxtParser {
    fn default() -> Self {
        Self::new()
    }
}
