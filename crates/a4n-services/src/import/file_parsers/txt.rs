use std::collections::HashMap;

use a4n_core::error::AppError;
use regex::Regex;

use super::{ParseResult, ParsedChapter};

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

    /// 查找首个匹配的章节标题行
    fn find_chapter_matches<'a>(&self, text: &'a str) -> Vec<(usize, &'a str)> {
        let mut matches: Vec<(usize, &str)> = Vec::new();
        for pattern in &self.chapter_patterns {
            for m in pattern.find_iter(text) {
                let line_start = line_start_at_or_before(text, m.start());
                let line_end = line_end_at_or_after(text, m.end());
                let title = normalize_chapter_title_line(&text[line_start..line_end]);
                matches.push((line_start, title));
            }
        }

        matches.sort_by_key(|(pos, _)| *pos);
        matches.dedup_by_key(|(pos, _)| *pos);

        matches
    }

    /// 解析 TXT 文本，按章节分割
    pub fn parse(&self, text: &str) -> Result<ParseResult, AppError> {
        let mut warnings = Vec::new();
        let chapters = self.split_into_chapters(text);

        if chapters.is_empty() {
            warnings.push("未检测到章节标记，整个文本作为一个章节处理。".to_string());
            return Ok(ParseResult {
                chapters: vec![ParsedChapter {
                    number: 1,
                    title: "完整内容".to_string(),
                    content: trim_imported_text(text).to_string(),
                    start_offset: 0,
                    end_offset: text.len(),
                }],
                detected_encoding: "utf-8".to_string(),
                metadata: HashMap::new(),
                warnings,
            });
        }

        Ok(ParseResult {
            chapters,
            detected_encoding: "utf-8".to_string(),
            metadata: HashMap::new(),
            warnings,
        })
    }

    fn split_into_chapters(&self, text: &str) -> Vec<ParsedChapter> {
        let matches = self.find_chapter_matches(text);

        if matches.is_empty() {
            return vec![];
        }

        let mut chapters = Vec::with_capacity(matches.len());

        for i in 0..matches.len() {
            let (start, title) = matches[i];
            let end = if i + 1 < matches.len() {
                matches[i + 1].0
            } else {
                text.len()
            };

            let content = trim_imported_text(&text[start..end]).to_string();
            chapters.push(ParsedChapter {
                number: (i + 1) as i32,
                title: title.to_string(),
                content,
                start_offset: start,
                end_offset: end,
            });
        }

        chapters
    }
}

pub(super) fn trim_imported_text(text: &str) -> &str {
    text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn normalize_chapter_title_line(line: &str) -> &str {
    let title = trim_imported_text(line);
    let hash_count = title.bytes().take_while(|byte| *byte == b'#').count();
    if (1..=6).contains(&hash_count) {
        return trim_imported_text(&title[hash_count..]);
    }
    title
}

pub(super) fn line_start_at_or_before(text: &str, offset: usize) -> usize {
    match text[..offset].rfind('\n') {
        Some(position) => position + 1,
        None => 0,
    }
}

pub(super) fn line_end_at_or_after(text: &str, offset: usize) -> usize {
    match text[offset..].find('\n') {
        Some(position) => offset + position,
        None => text.len(),
    }
}

impl Default for TxtParser {
    fn default() -> Self {
        Self::new()
    }
}
