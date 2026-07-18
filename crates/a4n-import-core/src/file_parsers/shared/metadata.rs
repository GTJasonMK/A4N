use std::collections::HashMap;

use super::extract_xml_text;

pub(in crate::file_parsers) fn extract_document_metadata(xml: &str) -> HashMap<String, String> {
    let mut metadata = HashMap::new();
    if let Some(title) = extract_xml_text(xml, "title") {
        metadata.insert("title".to_string(), title);
    }
    if let Some(creator) = extract_xml_text(xml, "creator") {
        metadata.insert("author".to_string(), creator);
    }
    metadata
}

#[cfg(test)]
mod tests {
    use super::extract_document_metadata;

    #[test]
    fn extracts_shared_title_and_creator_metadata() {
        let metadata = extract_document_metadata(
            "<metadata><dc:title>作品名</dc:title><dc:creator>作者名</dc:creator></metadata>",
        );

        assert_eq!(metadata.get("title").map(String::as_str), Some("作品名"));
        assert_eq!(metadata.get("author").map(String::as_str), Some("作者名"));
    }

    #[test]
    fn omits_missing_document_metadata() {
        assert!(extract_document_metadata("<metadata />").is_empty());
    }
}
