//! 外部文件解析器
//!
//! 支持 TXT（按章节标记分割）、EPUB 和 DOCX 格式。

mod docx;
mod epub;
mod shared {
    pub(super) mod entities;
    pub(super) mod metadata;
    pub(super) mod result;
    mod text;
}
mod txt;

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
}

/// 文件解析结果
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// 解析出的章节列表
    pub chapters: Vec<ParsedChapter>,
}
