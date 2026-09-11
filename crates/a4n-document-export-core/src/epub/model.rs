/// EPUB 元数据
#[derive(Debug, Clone)]
pub struct EpubMetadata {
    /// 书名
    pub title: String,
    /// 作者
    pub author: String,
    /// 语言（默认 zh-CN）
    pub language: String,
    /// 出版社
    pub publisher: Option<String>,
    /// 出版日期
    pub date: Option<String>,
    /// 简介
    pub description: Option<String>,
    /// 封面图片数据
    pub cover_image: Option<Vec<u8>>,
    /// 封面图片 MIME 类型
    pub cover_mime: Option<String>,
}

/// 章节内容
#[derive(Debug, Clone)]
pub struct EpubChapter {
    /// 章节标题
    pub title: String,
    /// XHTML 内容
    pub content: String,
    /// 章节序号
    pub order: usize,
}
