//! 项目 Host 输入与复制/重置路径的纯校验规则（无 DB、无运行时依赖）。

/// 一页制分页：page 从 1 起，计算 offset。
pub fn project_list_offset(page: u64, limit: u64) -> Result<u64, String> {
    page.checked_sub(1)
        .and_then(|page_index| page_index.checked_mul(limit))
        .ok_or_else(|| "项目分页参数过大".to_string())
}

/// 必填文本：trim 后非空。
pub fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(value)
    }
}

/// 可选文本：None 或空白视为空串。
pub fn optional_text(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_list_offset_uses_one_based_pages() {
        assert_eq!(project_list_offset(1, 20), Ok(0));
        assert_eq!(project_list_offset(3, 20), Ok(40));
    }

    #[test]
    fn project_list_offset_rejects_overflow() {
        let err = project_list_offset(u64::MAX, u64::MAX).expect_err("overflow");
        assert!(err.contains("分页参数过大"));
    }

    #[test]
    fn required_and_optional_text_trim() {
        assert_eq!(required_text(" 标题 ", "项目标题").unwrap(), "标题");
        assert!(required_text("  ", "项目标题").is_err());
        assert_eq!(optional_text(None), "");
        assert_eq!(optional_text(Some(" 简介 ")), "简介");
    }
}
