//! 反推 payload 文本校验。

use a4n_core::error::AppError;

pub(super) fn required_reverse_text<'a>(value: &'a str, field: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "反推任务 {field} 不能为空"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::required_reverse_text;

    #[test]
    fn trims_required_reverse_text() {
        assert_eq!(
            required_reverse_text("  内容  ", "instruction").expect("non-empty text"),
            "内容"
        );
    }

    #[test]
    fn rejects_blank_reverse_text_with_field_context() {
        for field in ["instruction", "content_sample"] {
            let error = required_reverse_text(" \n ", field).expect_err("blank text must fail");
            assert_eq!(
                error.to_string(),
                format!("参数无效：反推任务 {field} 不能为空")
            );
        }
    }
}
