use crate::error::AppError;

pub fn ensure_default_config_delete_allowed(
    config_kind: &str,
    is_default: bool,
) -> Result<(), AppError> {
    if is_default {
        return Err(AppError::InvalidArgument(format!(
            "不能删除默认{config_kind}，请先设置其它默认配置"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ensure_default_config_delete_allowed;

    #[test]
    fn rejects_default_configs() {
        for kind in [" LLM 配置", "图片配置", "视频配置"] {
            assert!(
                ensure_default_config_delete_allowed(kind, true)
                    .expect_err("default config deletion")
                    .to_string()
                    .contains(&format!("不能删除默认{kind}，请先设置其它默认配置"))
            );
        }
    }

    #[test]
    fn allows_non_default_configs() {
        for kind in [" LLM 配置", "图片配置", "视频配置"] {
            ensure_default_config_delete_allowed(kind, false).expect("non-default config deletion");
        }
    }
}
