use super::super::shared::decode_xml_entities;
use regex::Regex;

pub(super) fn extract_docx_document_text(xml: &str) -> String {
    let text_re = Regex::new(r"<w:t[^>]*>([^<]*)</w:t>").unwrap();
    let para_re = Regex::new(r"(?s)<w:p(?:\s|>).*?</w:p>").unwrap();
    let paragraphs = para_re
        .find_iter(xml)
        .filter_map(|para_match| {
            let para_text = text_re
                .captures_iter(para_match.as_str())
                .map(|cap| decode_xml_entities(&cap[1]))
                .collect::<String>();
            (!para_text.trim().is_empty()).then_some(para_text)
        })
        .collect::<Vec<_>>();

    paragraphs.join("\n\n")
}
