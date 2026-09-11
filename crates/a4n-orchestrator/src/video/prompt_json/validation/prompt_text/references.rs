use a4n_error::AppError;

pub fn validate_direct_prompt_text(text: &str, field: &str) -> Result<(), AppError> {
    if text.trim().is_empty() {
        return Ok(());
    }
    if let Some(term) = direct_prompt_internal_reference_term(text) {
        return Err(AppError::InvalidArgument(format!(
            "{field} 不能在自然语言提示词中写内部图片 ID 或内部字段“{term}”；只能按后端参考槽位顺序写“参考图 1”“参考图 2”，不要写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID"
        )));
    }
    Ok(())
}

pub(in crate::video) fn direct_prompt_internal_reference_term(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("image_id") {
        return Some("image_id");
    }
    if lower.contains("image id") {
        return Some("image id");
    }
    if lower.contains("available_assets") {
        return Some("available_assets");
    }
    if lower.contains("candidate_label") {
        return Some("candidate_label");
    }
    if lower.contains("source_display_label") {
        return Some("source_display_label");
    }
    if lower.contains("manual_library_asset") {
        return Some("manual_library_asset_*");
    }
    if lower.contains("asset_id") {
        return Some("asset_id");
    }
    if lower.contains("source_id") {
        return Some("source_id");
    }
    if text.contains("候选素材") {
        return Some("候选素材标签");
    }
    if contains_compact_reference_number(text, "参考图") {
        return Some("参考图后紧贴数字");
    }
    if contains_compact_reference_number(text, "图片") {
        return Some("图片后紧贴数字");
    }
    if contains_compact_reference_number(text, "素材图") {
        return Some("素材图后紧贴数字");
    }
    if text.contains("参考图 #") {
        return Some("参考图 #");
    }
    if text.contains("图片 #") {
        return Some("图片 #");
    }
    if text.contains("素材图 #") {
        return Some("素材图 #");
    }
    if lower.contains("图片id") {
        return Some("图片 ID");
    }
    if lower.contains("图片 id") {
        return Some("图片 ID");
    }
    if lower.contains("素材id") {
        return Some("素材 ID");
    }
    if lower.contains("素材 id") {
        return Some("素材 ID");
    }
    if lower.contains("参考图id") {
        return Some("参考图 ID");
    }
    if lower.contains("参考图 id") {
        return Some("参考图 ID");
    }
    None
}

fn contains_compact_reference_number(text: &str, marker: &str) -> bool {
    for (index, _) in text.match_indices(marker) {
        let after_marker = &text[index + marker.len()..];
        if after_marker
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

pub fn validate_reference_label_text(
    text: &str,
    field: &str,
    reference_count: usize,
) -> Result<(), AppError> {
    for (index, _) in text.match_indices("参考图") {
        let after_marker = &text[index + "参考图".len()..];
        let mut chars = after_marker.char_indices();
        let mut saw_whitespace = false;
        let digit_start = loop {
            match chars.next() {
                Some((_, ch)) if ch.is_whitespace() => {
                    saw_whitespace = true;
                }
                Some((index, ch)) if ch.is_ascii_digit() => break Some(index),
                _ => break None,
            }
        };
        let Some(digit_start) = digit_start else {
            continue;
        };
        if !saw_whitespace {
            return Err(AppError::InvalidArgument(format!(
                "{field} 的参考图编号必须按实际输入顺序写成“参考图 1”“参考图 2”，不能写成“参考图12”或内部图片 ID"
            )));
        }
        let digits = after_marker[digit_start..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(AppError::InvalidArgument(format!(
                "{field} 的参考图编号不能使用前导 0：参考图 {digits}"
            )));
        }
        let number = digits.parse::<usize>().map_err(|err| {
            AppError::InvalidArgument(format!("{field} 的参考图编号解析失败：{err}"))
        })?;
        if number == 0 || number > reference_count {
            let allowed = if reference_count == 0 {
                "当前没有绑定参考图槽位，不能写任何“参考图 N”".to_string()
            } else {
                format!(
                    "当前后端参考槽位绑定了 {reference_count} 张，只能写参考图 1 到参考图 {reference_count}"
                )
            };
            return Err(AppError::InvalidArgument(format!(
                "{field} 引用了不存在的参考图 {number}；{allowed}"
            )));
        }
    }
    Ok(())
}
