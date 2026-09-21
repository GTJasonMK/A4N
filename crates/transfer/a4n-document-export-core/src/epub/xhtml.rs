use crate::DocumentExportError;

mod body;

pub(super) fn required_stylesheet(stylesheet: &str) -> Result<&str, DocumentExportError> {
    let css = stylesheet.trim();
    if css.is_empty() {
        Err(DocumentExportError::InvalidEpub("EPUB 样式不能为空".into()))
    } else {
        Ok(css)
    }
}

pub(super) fn optional_metadata_tag(tag: &str, value: Option<&str>) -> String {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => format!("    <{tag}>{}</{tag}>\n", escape_xml(value)),
        None => String::new(),
    }
}

pub(super) fn build_chapter_xhtml(title: &str, content: &str) -> String {
    let body = body::render_paragraphs(content);

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="zh-CN">
<head>
  <title>{}</title>
  <link rel="stylesheet" type="text/css" href="style.css"/>
</head>
<body>
  <h1>{}</h1>
{}
</body>
</html>"#,
        escape_xml(title),
        escape_xml(title),
        body,
    )
}

pub(super) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
