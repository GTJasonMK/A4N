use std::collections::HashSet;

use a4n_error::AppError;

pub(super) fn validate_prompt_update_content(content: &str) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument("提示词内容不能为空".to_string()));
    }
    Ok(())
}

pub(super) fn normalize_prompt_update_author(author: &str) -> Result<String, AppError> {
    let author = author.trim();
    if author.is_empty() {
        return Err(AppError::InvalidArgument(
            "提示词更新作者不能为空".to_string(),
        ));
    }
    Ok(author.to_string())
}

pub(super) fn ensure_unique_prompt_update_names(
    updates: &[(String, String)],
) -> Result<(), AppError> {
    let mut names = HashSet::new();
    for (name, _) in updates {
        if !names.insert(name.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "提示词批量更新包含重复 name：{name}"
            )));
        }
    }
    Ok(())
}
