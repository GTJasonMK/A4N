use crate::ImportTransformError;

use super::super::{ParseResult, TxtParser};

pub(in crate::file_parsers) fn parse_extracted_document(
    text_content: &str,
    document_title: Option<&str>,
    source_title: &str,
) -> Result<ParseResult, ImportTransformError> {
    let document_title = document_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(source_title);
    TxtParser::new().parse(text_content, document_title)
}

#[cfg(test)]
mod tests {
    use super::parse_extracted_document;

    #[test]
    fn uses_extracted_document_title() {
        let result = parse_extracted_document("无章节标题正文", Some("作品"), "源文件名")
            .expect("extracted document should parse");

        assert_eq!(result.chapters[0].title, "作品");
    }
}
