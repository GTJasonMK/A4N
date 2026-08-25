//! 主题业务规则（无 DB）。

/// 主题名称 trim 后非空。
pub(super) fn normalize_theme_name(name: String) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("主题名称不能为空".to_string());
    }
    Ok(name.to_string())
}

/// 系统默认主题与当前激活主题不可删除。
pub(super) fn ensure_theme_can_be_deleted(
    is_system_default: bool,
    is_active: bool,
) -> Result<(), String> {
    if is_system_default {
        return Err("系统默认主题不能删除".to_string());
    }
    if is_active {
        return Err("正在使用的主题不能删除，请先激活其他主题".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_theme_name_trims_and_rejects_empty() {
        assert_eq!(
            normalize_theme_name("  Quiet Dark  ".to_string()).unwrap(),
            "Quiet Dark"
        );
        let error = normalize_theme_name("   ".to_string()).expect_err("empty");
        assert!(error.contains("主题名称不能为空"));
    }

    #[test]
    fn delete_theme_rejects_system_default_or_active() {
        let error = ensure_theme_can_be_deleted(true, false).expect_err("system");
        assert!(error.contains("系统默认主题不能删除"));
        let error = ensure_theme_can_be_deleted(false, true).expect_err("active");
        assert!(error.contains("正在使用的主题不能删除"));
        ensure_theme_can_be_deleted(false, false).expect("ok");
    }
}
