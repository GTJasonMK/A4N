use a4n_api_contract::optimize::OptimizationPatchOperation;
use a4n_core::error::AppError;
use a4n_core::source_spans::source_text_hash;

use super::contracts::OptimizationPatch;
use super::validation::{normalize_optimization_patch, validate_optimization_patches};

pub fn optimization_content_hash(content: &str) -> String {
    source_text_hash(content)
}

pub fn apply_optimization_patches_to_content(
    content: &str,
    base_content_hash: &str,
    patches: &[OptimizationPatch],
) -> Result<String, AppError> {
    let expected_hash = base_content_hash.trim();
    if expected_hash.is_empty() {
        return Err(AppError::InvalidArgument(
            "base_content_hash 不能为空".to_string(),
        ));
    }
    let current_hash = optimization_content_hash(content);
    if current_hash != expected_hash {
        return Err(AppError::InvalidState {
            current: "content_changed".to_string(),
            operation: format!(
                "应用局部补丁：当前正文 hash 为 {current_hash}，不等于补丁基准 {expected_hash}"
            ),
        });
    }
    validate_optimization_patches(content, patches)?;

    let mut next_content = content.to_string();
    for raw_patch in patches {
        let patch = normalize_optimization_patch(raw_patch.clone());
        let occurrences = next_content.matches(&patch.old_text).count();
        if occurrences != 1 {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} 的 old_text 在当前正文中匹配 {} 次，必须唯一匹配",
                patch.patch_id, occurrences
            )));
        }
        let replacement = match patch.operation {
            OptimizationPatchOperation::Replace => patch.new_text.clone(),
            OptimizationPatchOperation::Delete => String::new(),
            OptimizationPatchOperation::InsertBefore => {
                format!("{}{}", patch.new_text, patch.old_text)
            }
            OptimizationPatchOperation::InsertAfter => {
                format!("{}{}", patch.old_text, patch.new_text)
            }
        };
        next_content = next_content.replacen(&patch.old_text, &replacement, 1);
    }
    Ok(next_content)
}
