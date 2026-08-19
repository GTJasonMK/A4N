use super::super::super::shared::entities::decode_xml_entities;
use super::path::{is_html_path, resolve_epub_href};
use regex::Regex;
use std::collections::HashMap;

pub(in crate::file_parsers::epub) struct EpubHtmlEntry {
    pub(in crate::file_parsers::epub) path: String,
    pub(in crate::file_parsers::epub) text: String,
}

pub(in crate::file_parsers::epub) fn extract_epub_spine_paths(
    opf: &str,
    opf_path: &str,
) -> Vec<String> {
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
    itemref_re
        .find_iter(opf)
        .filter_map(|itemref| extract_xml_attr(itemref.as_str(), "idref"))
        .filter_map(|idref| manifest.get(&idref).cloned())
        .collect()
}

fn extract_xml_attr(tag: &str, attr: &str) -> Option<String> {
    let double_quote_re =
        Regex::new(&format!(r#"\b{}\s*=\s*"([^"]*)""#, regex::escape(attr))).ok()?;
    if let Some(value) = double_quote_re
        .captures(tag)
        .and_then(|cap| cap.get(1))
        .map(|m| decode_xml_entities(m.as_str()))
    {
        return Some(value);
    }
    Regex::new(&format!(r#"\b{}\s*=\s*'([^']*)'"#, regex::escape(attr)))
        .ok()?
        .captures(tag)
        .and_then(|cap| cap.get(1))
        .map(|m| decode_xml_entities(m.as_str()))
}
