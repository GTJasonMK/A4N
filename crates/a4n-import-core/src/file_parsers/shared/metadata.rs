use super::text::extract_xml_text;

pub(in crate::file_parsers) fn extract_document_title(xml: &str) -> Option<String> {
    extract_xml_text(xml, "title")
}

#[cfg(test)]
mod tests {
    use super::extract_document_title;

    #[test]
    fn extracts_shared_document_title() {
        let title = extract_document_title(
            "<metadata><dc:title>作品名</dc:title><dc:creator>作者名</dc:creator></metadata>",
        );

        assert_eq!(title.as_deref(), Some("作品名"));
    }

    #[test]
    fn omits_missing_document_title() {
        assert!(extract_document_title("<metadata />").is_none());
    }
}
