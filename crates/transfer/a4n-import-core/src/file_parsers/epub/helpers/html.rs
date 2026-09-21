use super::super::super::shared::entities::decode_xml_entities;
use regex::Regex;

#[cfg(test)]
mod tests;

pub(in crate::file_parsers::epub) fn strip_html_tags(html: &str) -> String {
    let body_re = Regex::new(r"(?is)<body\b[^>]*>(.*?)</body>").unwrap();
    let html = body_re
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map_or(html, |body| body.as_str());
    let block_tag_re = Regex::new(r"(?i)<\s*/?\s*(address|article|aside|blockquote|br|dd|div|dl|dt|figcaption|figure|footer|h[1-6]|header|hr|li|main|nav|ol|p|pre|section|table|tbody|td|tfoot|th|thead|tr|ul)\b[^>]*>").unwrap();
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    let with_lines = block_tag_re.replace_all(html, "\n");
    let text = decode_xml_entities(&tag_re.replace_all(&with_lines, " "));
    let ws_re = Regex::new(r"[^\S\n]+").unwrap();
    text.lines()
        .filter_map(|line| {
            let line = ws_re.replace_all(line, " ");
            let line = line.trim();
            (!line.is_empty()).then(|| line.to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
}
