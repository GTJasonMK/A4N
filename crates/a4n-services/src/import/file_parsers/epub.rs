use std::collections::HashMap;

use a4n_core::error::AppError;
use regex::Regex;

use super::shared::{decode_xml_entities, extract_xml_text};
use super::{ParseResult, TxtParser};

/// EPUB 文件解析器
///
/// 使用 zip crate 解包 EPUB 容器，提取 XHTML 文本内容。
pub struct EpubParser;

struct EpubHtmlEntry {
    path: String,
    text: String,
}

impl EpubParser {
    pub fn new() -> Self {
        Self
    }

    /// 从 EPUB 字节数据解析文本内容
    pub fn parse(&self, data: &[u8]) -> Result<ParseResult, AppError> {
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| AppError::InvalidArgument(format!("无法打开 EPUB 文件: {e}")))?;

        let mut metadata = HashMap::new();
        let mut warnings = Vec::new();
        let mut html_entries = Vec::new();
        let mut spine_paths = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::InvalidArgument(format!("无法读取 EPUB 条目: {e}")))?;

            let name = normalize_epub_path(file.name());

            if name.ends_with(".opf") {
                let content = std::io::read_to_string(&mut file).map_err(|e| {
                    AppError::InvalidArgument(format!("无法读取 EPUB 元数据文件 {name}: {e}"))
                })?;
                if let Some(title) = extract_xml_text(&content, "title") {
                    metadata.insert("title".to_string(), title);
                }
                if let Some(creator) = extract_xml_text(&content, "creator") {
                    metadata.insert("author".to_string(), creator);
                }
                spine_paths.extend(extract_epub_spine_paths(&content, &name));
            }

            if name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm") {
                let html = std::io::read_to_string(&mut file).map_err(|e| {
                    AppError::InvalidArgument(format!("无法读取 EPUB 内容文件 {name}: {e}"))
                })?;
                let text = strip_html_tags(&html);
                if !text.trim().is_empty() {
                    html_entries.push(EpubHtmlEntry { path: name, text });
                }
            }
        }

        let has_content = !html_entries.is_empty();
        if !has_content {
            warnings.push("EPUB 中未找到 XHTML/HTML 内容文件。".to_string());
        }

        let mut text_content = String::new();
        for text in order_epub_html_entries(html_entries, &spine_paths) {
            text_content.push_str(&text);
            text_content.push('\n');
        }

        let txt_parser = TxtParser::new();
        let mut result = txt_parser.parse(&text_content)?;
        result.detected_encoding = "utf-8".to_string();
        result.metadata = metadata;
        result.warnings.extend(warnings);

        Ok(result)
    }
}

fn order_epub_html_entries(entries: Vec<EpubHtmlEntry>, spine_paths: &[String]) -> Vec<String> {
    if spine_paths.is_empty() {
        return entries.into_iter().map(|entry| entry.text).collect();
    }

    let mut used = vec![false; entries.len()];
    let mut ordered = Vec::with_capacity(entries.len());

    for spine_path in spine_paths {
        if let Some((index, entry)) = entries
            .iter()
            .enumerate()
            .find(|(index, entry)| !used[*index] && entry.path == *spine_path)
        {
            used[index] = true;
            ordered.push(entry.text.clone());
        }
    }

    for (index, entry) in entries.into_iter().enumerate() {
        if !used[index] {
            ordered.push(entry.text);
        }
    }

    ordered
}

fn extract_epub_spine_paths(opf: &str, opf_path: &str) -> Vec<String> {
    let item_re = Regex::new(r"(?is)<item\b[^>]*>").unwrap();
    let itemref_re = Regex::new(r"(?is)<itemref\b[^>]*>").unwrap();
    let mut manifest: HashMap<String, String> = HashMap::new();

    for item in item_re.find_iter(opf) {
        let tag = item.as_str();
        let Some(id) = extract_xml_attr(tag, "id") else {
            continue;
        };
        let Some(href) = extract_xml_attr(tag, "href") else {
            continue;
        };
        let media_type = extract_xml_attr(tag, "media-type").unwrap_or_default();
        if media_type.contains("xhtml") || media_type.contains("html") || is_html_path(&href) {
            manifest.insert(id, resolve_epub_href(opf_path, &href));
        }
    }

    let mut paths = Vec::new();
    for itemref in itemref_re.find_iter(opf) {
        let Some(idref) = extract_xml_attr(itemref.as_str(), "idref") else {
            continue;
        };
        if let Some(path) = manifest.get(&idref) {
            paths.push(path.clone());
        }
    }

    paths
}

fn extract_xml_attr(tag: &str, attr: &str) -> Option<String> {
    let double_quote_pattern = format!(r#"\b{}\s*=\s*"([^"]*)""#, regex::escape(attr));
    let double_quote_re = Regex::new(&double_quote_pattern).ok()?;
    if let Some(value) = double_quote_re
        .captures(tag)
        .and_then(|cap| cap.get(1))
        .map(|m| decode_xml_entities(m.as_str()))
    {
        return Some(value);
    }

    let single_quote_pattern = format!(r#"\b{}\s*=\s*'([^']*)'"#, regex::escape(attr));
    let single_quote_re = Regex::new(&single_quote_pattern).ok()?;
    single_quote_re
        .captures(tag)
        .and_then(|cap| cap.get(1))
        .map(|m| decode_xml_entities(m.as_str()))
}

pub(super) fn resolve_epub_href(opf_path: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or(href);
    let href = href.replace('\\', "/");
    let href = href.trim_start_matches("./");
    if href.starts_with('/') {
        return normalize_epub_path(href.trim_start_matches('/'));
    }

    match opf_path.rsplit_once('/') {
        Some((base, _)) if !base.is_empty() => normalize_epub_path(&format!("{base}/{href}")),
        _ => normalize_epub_path(href),
    }
}

fn normalize_epub_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/").to_lowercase()
}

fn is_html_path(path: &str) -> bool {
    let path = path.to_lowercase();
    path.ends_with(".xhtml") || path.ends_with(".html") || path.ends_with(".htm")
}

/// 去除 HTML 标签，保留文本内容
pub(super) fn strip_html_tags(html: &str) -> String {
    let block_tag_re = Regex::new(
        r"(?i)<\s*/?\s*(address|article|aside|blockquote|br|dd|div|dl|dt|figcaption|figure|footer|h[1-6]|header|hr|li|main|nav|ol|p|pre|section|table|tbody|td|tfoot|th|thead|tr|ul)\b[^>]*>",
    )
    .unwrap();
    let tag_re = Regex::new(r"<[^>]+>").unwrap();

    let with_lines = block_tag_re.replace_all(html, "\n");
    let mut text = tag_re.replace_all(&with_lines, " ").to_string();
    text = decode_xml_entities(&text);
    normalize_extracted_text(&text)
}

fn normalize_extracted_text(text: &str) -> String {
    let ws_re = Regex::new(r"[^\S\n]+").unwrap();
    text.lines()
        .filter_map(|line| {
            let line = ws_re.replace_all(line, " ");
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
