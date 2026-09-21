use super::extract_xml_text;

#[test]
fn decodes_xml_entities() {
    let xml = "<metadata><dc:title>A4N &amp; Co</dc:title></metadata>";
    assert_eq!(extract_xml_text(xml, "title"), Some("A4N & Co".to_string()));
}
