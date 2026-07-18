use super::EpubBuilder;
use crate::epub::test_support::minimal_metadata;
use crate::epub::{EpubMetadata, builtin_epub_stylesheet};

#[test]
fn omits_missing_publication_metadata() {
    let builder = EpubBuilder::new(minimal_metadata(), builtin_epub_stylesheet().to_string());

    let opf = builder.build_opf().unwrap();

    assert!(!opf.contains("<dc:date>"));
    assert!(!opf.contains("<dc:publisher>"));
    assert!(!opf.contains("2026-01-01"));
    assert!(opf.contains(r#"<meta property="dcterms:modified">"#));
}

#[test]
fn writes_explicit_publication_metadata() {
    let metadata = EpubMetadata {
        publisher: Some("A4N & Co".into()),
        date: Some("2026-05-26".into()),
        description: Some("A < B & C".into()),
        ..minimal_metadata()
    };
    let builder = EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string());

    let opf = builder.build_opf().unwrap();

    assert!(opf.contains("<dc:date>2026-05-26</dc:date>"));
    assert!(opf.contains("<dc:publisher>A4N &amp; Co</dc:publisher>"));
    assert!(opf.contains("<dc:description>A &lt; B &amp; C</dc:description>"));
}
