//! 外部文件解析器
//!
//! 支持 TXT（按章节标记分割）、EPUB 和 DOCX 格式。

mod docx;
mod epub;
mod shared;
#[cfg(test)]
mod tests;
mod txt;

use std::collections::HashMap;

pub use self::docx::DocxParser;
pub use self::epub::EpubParser;
pub use self::txt::TxtParser;

/// 解析后的章节
#[derive(Debug, Clone)]
pub struct ParsedChapter {
    /// 章节序号（从1开始）
    pub number: i32,
    /// 章节标题
    pub title: String,
    /// 章节正文
    pub content: String,
    /// 原始位置（字节偏移）
    pub start_offset: usize,
    /// 原始位置（字节偏移结束）
    pub end_offset: usize,
}

/// 文件解析结果
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// 解析出的章节列表
    pub chapters: Vec<ParsedChapter>,
    /// 检测到的文件编码
    pub detected_encoding: String,
    /// 元数据（标题、作者等）
    pub metadata: HashMap<String, String>,
    /// 解析警告
    pub warnings: Vec<String>,
}
