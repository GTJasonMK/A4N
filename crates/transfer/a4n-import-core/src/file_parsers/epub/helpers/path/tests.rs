use super::resolve_epub_href;

#[test]
fn normalizes_dot_segments() {
    assert_eq!(
        resolve_epub_href("OEBPS/package/content.opf", "../Text/./Ch1.XHTML#section"),
        "oebps/text/ch1.xhtml"
    );
}
