fn ensure_manga_page_panels_match_current_source(
    page_prompt: &MangaPagePrompt,
    current_content: &str,
) -> Result<(), AppError> {
    let current_hash = source_content_sha256(current_content);
    let layout = manga_page_generation_layout(
        page_prompt.id,
        page_prompt.page_number,
        page_prompt.layout.as_ref(),
    )
    .map_err(AppError::InvalidArgument)?;
    if layout.source.content_sha256.trim().is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "漫画提示词 #{} 缺少章节正文指纹，请重新生成漫画提示词",
            page_prompt.id
        )));
    }
    if layout.source.content_sha256 != current_hash {
        return Err(AppError::InvalidArgument(format!(
            "漫画提示词 #{} 基于旧章节正文，请重新生成漫画提示词",
            page_prompt.id
        )));
    }
    Ok(())
}

fn image_source_metadata(content: &str) -> Value {
    source_metadata(ChapterSourceContext { content })
}

fn merge_source_metadata(
    metadata_json: Option<Value>,
    source_metadata: Option<Value>,
    preserve_existing_source: bool,
) -> Option<Value> {
    let Some(source_metadata) = source_metadata else {
        return metadata_json;
    };
    match metadata_json {
        Some(Value::Object(mut map)) => {
            let key = if preserve_existing_source && map.contains_key("source") {
                "chapter_source"
            } else {
                "source"
            };
            map.insert(key.into(), source_metadata);
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "value": value,
            "source": source_metadata,
        })),
        None => Some(json!({
            "source": source_metadata,
        })),
    }
}

fn merge_direct_image_metadata(base: Option<Value>, extra: Option<Value>) -> Option<Value> {
    match (base, extra) {
        (Some(Value::Object(mut base)), Some(Value::Object(extra))) => {
            base.extend(extra);
            Some(Value::Object(base))
        }
        (Some(Value::Object(mut base)), Some(extra)) => {
            base.insert("extra".to_string(), extra);
            Some(Value::Object(base))
        }
        (Some(base), Some(Value::Object(mut extra))) => {
            extra.insert("base".to_string(), base);
            Some(Value::Object(extra))
        }
        (Some(base), Some(extra)) => Some(json!({
            "base": base,
            "extra": extra,
        })),
        (Some(base), None) => Some(base),
        (None, Some(extra)) => Some(extra),
        (None, None) => None,
    }
}

pub(super) fn check_progress_cancelled(
    progress: &ImageGenerationProgress<'_>,
) -> Result<(), AppError> {
    progress.check_cancelled().map_err(|_| AppError::Cancelled)
}

fn require_positive(value: i64, label: &str) -> Result<(), AppError> {
    if value <= 0 {
        Err(AppError::InvalidArgument(format!("{label} 必须大于 0")))
    } else {
        Ok(())
    }
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label} 不能为空")))
    } else {
        Ok(value)
    }
}

fn required_image_size(width: Option<i32>, height: Option<i32>) -> Result<(i32, i32), AppError> {
    let width = width.ok_or_else(|| AppError::InvalidArgument("图片宽度不能为空".to_string()))?;
    let height = height.ok_or_else(|| AppError::InvalidArgument("图片高度不能为空".to_string()))?;
    require_positive(i64::from(width), "图片宽度")?;
    require_positive(i64::from(height), "图片高度")?;
    Ok((width, height))
}
