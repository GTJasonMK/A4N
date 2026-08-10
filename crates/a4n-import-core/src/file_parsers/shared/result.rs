use std::collections::HashMap;

use crate::ImportTransformError;

use super::super::{ParseResult, TxtParser};

pub(in crate::file_parsers) fn parse_extracted_document(
    text_content: &str,
    metadata: HashMap<String, String>,
    warnings: Vec<String>,
    source_title: &str,
) -> Result<ParseResult, ImportTransformError> {
    let document_title = metadata
        .get("title")
        .map(String::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(source_title);
    let mut result = TxtParser::new().parse(text_content, document_title)?;
    result.metadata = metadata;
    result.warnings.extend(warnings);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::parse_extracted_document;

    #[test]
    fn adapts_extracted_document_metadata_and_warnings() {
        let metadata = HashMap::from([("title".to_string(), "作品".to_string())]);
        let result = parse_extracted_document(
            "无章节标题正文",
            metadata,
            vec!["格式 warning".to_string()],
            "源文件名",
        )
        .expect("extracted document should parse");

        assert_eq!(result.detected_encoding, "utf-8");
        assert_eq!(result.chapters[0].title, "作品");
        assert_eq!(
            result.metadata.get("title").map(String::as_str),
            Some("作品")
        );
        assert_eq!(
            result.warnings,
            [
                "未检测到章节标记，整个文本作为一个章节处理。",
                "格式 warning",
            ]
        );
    }
}
