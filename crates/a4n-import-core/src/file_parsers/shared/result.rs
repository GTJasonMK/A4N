use std::collections::HashMap;

use crate::ImportTransformError;

use super::super::{ParseResult, TxtParser};

pub(in crate::file_parsers) fn parse_extracted_document(
    text_content: &str,
    metadata: HashMap<String, String>,
    warnings: Vec<String>,
) -> Result<ParseResult, ImportTransformError> {
    let mut result = TxtParser::new().parse(text_content)?;
    result.detected_encoding = "utf-8".to_string();
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
        let result =
            parse_extracted_document("无章节标题正文", metadata, vec!["格式 warning".to_string()])
                .expect("extracted document should parse");

        assert_eq!(result.detected_encoding, "utf-8");
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
