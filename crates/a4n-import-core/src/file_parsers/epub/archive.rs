use crate::ImportTransformError;

use super::helpers::ordering::order_epub_html_entries;

mod entries;

use entries::scan_epub_entries;

pub(super) struct EpubArchiveContent {
    pub(super) title: Option<String>,
    pub(super) text_content: String,
}

pub(super) fn read_epub_archive(data: &[u8]) -> Result<EpubArchiveContent, ImportTransformError> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ImportTransformError::InvalidDocument(format!("无法打开 EPUB 文件: {e}")))?;

    let entries = scan_epub_entries(&mut archive)?;
    if entries.html_entries.is_empty() {
        return Err(ImportTransformError::InvalidDocument(
            "EPUB 中未找到 XHTML/HTML 内容文件".to_string(),
        ));
    }

    let mut text_content = String::new();
    for text in order_epub_html_entries(entries.html_entries, &entries.spine_paths) {
        text_content.push_str(&text);
        text_content.push('\n');
    }

    Ok(EpubArchiveContent {
        title: entries.title,
        text_content,
    })
}
