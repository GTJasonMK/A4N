use std::io::{Read, Seek};

use crate::ImportTransformError;

use super::super::super::shared::metadata::extract_document_title;
use super::super::helpers::html::strip_html_tags;
use super::super::helpers::path::normalize_epub_path;
use super::super::helpers::spine::{EpubHtmlEntry, extract_epub_spine_paths};

pub(super) struct EpubArchiveEntries {
    pub(super) title: Option<String>,
    pub(super) html_entries: Vec<EpubHtmlEntry>,
    pub(super) spine_paths: Vec<String>,
}

pub(super) fn scan_epub_entries<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<EpubArchiveEntries, ImportTransformError> {
    let mut title = None;
    let mut html_entries = Vec::new();
    let mut spine_paths = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            ImportTransformError::InvalidDocument(format!("无法读取 EPUB 条目: {e}"))
        })?;
        let name = normalize_epub_path(file.name());

        if name.ends_with(".opf") {
            let content = std::io::read_to_string(&mut file).map_err(|e| {
                ImportTransformError::InvalidDocument(format!(
                    "无法读取 EPUB 元数据文件 {name}: {e}"
                ))
            })?;
            if let Some(value) = extract_document_title(&content) {
                title = Some(value);
            }
            spine_paths.extend(extract_epub_spine_paths(&content, &name));
        }

        if name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm") {
            let html = std::io::read_to_string(&mut file).map_err(|e| {
                ImportTransformError::InvalidDocument(format!("无法读取 EPUB 内容文件 {name}: {e}"))
            })?;
            let text = strip_html_tags(&html);
            if !text.trim().is_empty() {
                html_entries.push(EpubHtmlEntry { path: name, text });
            }
        }
    }

    Ok(EpubArchiveEntries {
        title,
        html_entries,
        spine_paths,
    })
}
