//! 导出 Host 输入的纯校验规则（无 DB、无运行时依赖）。

use crate::project::required_optional_text;

/// 规范化批量导出格式列表：去空白、小写别名、去重。
pub fn normalize_batch_export_formats(formats: &[String]) -> Result<Vec<&'static str>, String> {
    if formats.is_empty() {
        return Err("批量导出格式不能为空".to_string());
    }

    let mut normalized = Vec::with_capacity(formats.len());
    for format in formats {
        let format = format.trim();
        if format.is_empty() {
            return Err("批量导出格式不能为空".to_string());
        }
        let normalized_format = match format.to_ascii_lowercase().as_str() {
            "txt" => "txt",
            "markdown" | "md" => "markdown",
            "epub" => "epub",
            "json" => "json",
            "pdf" => "pdf",
            _ => return Err(format!("不支持的导出格式: {format}")),
        };
        if normalized.contains(&normalized_format) {
            return Err(format!("批量导出格式重复: {normalized_format}"));
        }
        normalized.push(normalized_format);
    }
    Ok(normalized)
}

/// 批量导出标题/作者：仅在需要元数据的格式时要求非空。
pub fn batch_export_metadata_fields<'a>(
    title: Option<&'a str>,
    author: Option<&'a str>,
    needs_metadata: bool,
) -> Result<(Option<&'a str>, Option<&'a str>), String> {
    if !needs_metadata {
        return Ok((None, None));
    }

    Ok((
        Some(required_optional_text(title, "批量导出标题")?),
        Some(required_optional_text(author, "批量导出作者")?),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_batch_export_formats_rejects_empty_unknown_and_duplicate() {
        for formats in [
            Vec::<String>::new(),
            vec![" ".to_string()],
            vec!["docx".to_string()],
            vec!["md".to_string(), "markdown".to_string()],
            vec!["epub".to_string(), "EPUB".to_string()],
        ] {
            normalize_batch_export_formats(&formats).expect_err("formats should be rejected");
        }
    }

    #[test]
    fn normalize_batch_export_formats_accepts_supported_aliases() {
        let formats = vec![
            "txt".to_string(),
            "MD".to_string(),
            "epub".to_string(),
            "json".to_string(),
            "pdf".to_string(),
        ];
        assert_eq!(
            normalize_batch_export_formats(&formats),
            Ok(vec!["txt", "markdown", "epub", "json", "pdf"])
        );
    }

    #[test]
    fn batch_export_metadata_fields_optional_when_not_needed() {
        assert_eq!(
            batch_export_metadata_fields(None, None, false),
            Ok((None, None))
        );
    }

    #[test]
    fn batch_export_metadata_fields_requires_title_and_author() {
        let err = batch_export_metadata_fields(None, Some("作者"), true).unwrap_err();
        assert!(err.contains("批量导出标题"));
        let err = batch_export_metadata_fields(Some("书名"), Some("  "), true).unwrap_err();
        assert!(err.contains("批量导出作者"));
        assert_eq!(
            batch_export_metadata_fields(Some(" 书名 "), Some(" 作者 "), true),
            Ok((Some("书名"), Some("作者")))
        );
    }
}
