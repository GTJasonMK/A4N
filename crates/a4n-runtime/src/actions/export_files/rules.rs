use super::contracts::BatchExportInput;

pub(super) fn normalize_batch_formats(formats: &[String]) -> Result<Vec<&'static str>, String> {
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

pub(super) fn batch_export_metadata(
    input: &BatchExportInput,
    needs_metadata: bool,
) -> Result<(Option<&str>, Option<&str>), String> {
    if !needs_metadata {
        return Ok((None, None));
    }

    Ok((
        Some(required_optional_text(
            input.title.as_deref(),
            "批量导出标题",
        )?),
        Some(required_optional_text(
            input.author.as_deref(),
            "批量导出作者",
        )?),
    ))
}

pub(super) fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value)
    }
}

fn required_optional_text<'a>(value: Option<&'a str>, label: &str) -> Result<&'a str, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label}不能为空"))
}
