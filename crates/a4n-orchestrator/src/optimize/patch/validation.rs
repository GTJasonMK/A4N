use std::collections::HashSet;

use a4n_api_contract::optimize::OptimizationPatchOperation;
use a4n_error::AppError;

use super::super::{
    OPTIMIZATION_PATCH_MAX_COUNT, OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS,
    OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS,
};
use super::contracts::OptimizationPatch;

pub(super) fn normalize_optimization_patch(mut patch: OptimizationPatch) -> OptimizationPatch {
    patch.patch_id = patch.patch_id.trim().to_string();
    patch.issue_id = patch
        .issue_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    patch.old_text = patch.old_text.trim().to_string();
    patch.new_text = patch.new_text.trim().to_string();
    patch.reason = patch.reason.trim().to_string();
    patch.risk = patch
        .risk
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    patch
}

pub(super) fn validate_optimization_patches(
    chapter_content: &str,
    patches: &[OptimizationPatch],
) -> Result<(), AppError> {
    if patches.is_empty() {
        return Err(AppError::InvalidArgument(
            "content_optimization_patch.patches 不能为空".to_string(),
        ));
    }
    if patches.len() > OPTIMIZATION_PATCH_MAX_COUNT {
        return Err(AppError::InvalidArgument(format!(
            "content_optimization_patch.patches 最多允许 {} 个",
            OPTIMIZATION_PATCH_MAX_COUNT
        )));
    }
    let mut patch_ids = HashSet::new();
    let chapter_chars = chapter_content.chars().count();
    for (index, raw_patch) in patches.iter().enumerate() {
        let patch = normalize_optimization_patch(raw_patch.clone());
        if patch.patch_id.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "patches[{index}].patch_id 不能为空"
            )));
        }
        if !patch_ids.insert(patch.patch_id.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "patch_id 重复：{}",
                patch.patch_id
            )));
        }
        if patch.old_text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} old_text 不能为空",
                patch.patch_id
            )));
        }
        if patch.operation == OptimizationPatchOperation::Delete && !patch.new_text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} operation=delete 时 new_text 必须为空字符串",
                patch.patch_id
            )));
        }
        if patch.operation != OptimizationPatchOperation::Delete && patch.new_text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} operation={} 时 new_text 不能为空",
                patch.patch_id, patch.operation
            )));
        }
        if patch.operation == OptimizationPatchOperation::Replace
            && patch.old_text == patch.new_text
        {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} operation=replace 时 new_text 不能与 old_text 相同",
                patch.patch_id
            )));
        }
        if patch.reason.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} reason 不能为空",
                patch.patch_id
            )));
        }
        let old_chars = patch.old_text.chars().count();
        if old_chars > OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} old_text 过长：{} 字，最大 {} 字",
                patch.patch_id, old_chars, OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS
            )));
        }
        let new_chars = patch.new_text.chars().count();
        if patch.operation != OptimizationPatchOperation::Delete
            && new_chars > OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS
        {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} new_text 过长：{} 字，最大 {} 字",
                patch.patch_id, new_chars, OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS
            )));
        }
        if patch.old_text.trim() == chapter_content.trim()
            || (chapter_chars > 4000 && old_chars * 2 > chapter_chars)
        {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} 范围过大，疑似整章或大段重写",
                patch.patch_id
            )));
        }
        let occurrences = chapter_content.matches(&patch.old_text).count();
        if occurrences != 1 {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} old_text 在当前正文中匹配 {} 次，必须唯一匹配",
                patch.patch_id, occurrences
            )));
        }
    }
    Ok(())
}
