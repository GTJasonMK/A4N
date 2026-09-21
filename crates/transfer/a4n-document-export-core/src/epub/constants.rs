pub(super) const CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

const BUILTIN_EPUB_STYLESHEET: &str = r#"body {
  font-family: serif;
  line-height: 1.8;
  margin: 1em 2em;
}
h1 {
  text-align: center;
  font-size: 1.5em;
  margin: 1em 0;
}
p {
  text-indent: 2em;
  margin: 0.5em 0;
}"#;

pub fn builtin_epub_stylesheet() -> &'static str {
    BUILTIN_EPUB_STYLESHEET
}
