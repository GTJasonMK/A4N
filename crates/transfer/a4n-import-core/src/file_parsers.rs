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

pub use a4n_import_contract::ImportParsedChapter as ParsedChapter;

/// 文件解析结果
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// 解析出的章节列表
    pub chapters: Vec<ParsedChapter>,
}
