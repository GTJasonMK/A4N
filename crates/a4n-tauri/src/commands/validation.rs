#[cfg(test)]
pub(super) fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value)
    }
}

pub(super) fn required_positive_u64(value: Option<u64>, label: &str) -> Result<u64, String> {
    value
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label}必须为正整数"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_text_rejects_blank_values_and_trims_valid_values() {
        for value in ["", " \n\t "] {
            let err = required_text(value, "项目标题").expect_err("text should be required");

            assert!(err.contains("项目标题不能为空"));
        }

        assert_eq!(required_text(" 长篇小说 ", "项目类型"), Ok("长篇小说"));
    }

    #[test]
    fn positive_number_helpers_reject_missing_or_non_positive_values() {
        for value in [None, Some(0)] {
            let err = required_positive_u64(value, "page").expect_err("page should be positive");

            assert!(err.contains("page必须为正整数"));
        }

        assert_eq!(required_positive_u64(Some(20), "page_size"), Ok(20));
    }
}
