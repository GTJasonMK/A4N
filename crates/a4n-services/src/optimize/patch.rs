use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    OPTIMIZATION_PATCH_MAX_COUNT, OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS,
    OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationPatch {
    pub patch_id: String,
    pub issue_id: Option<String>,
    pub operation: String,
    pub old_text: String,
    pub new_text: String,
    pub reason: String,
    pub risk: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OptimizationPatchProposal {
    pub session_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OptimizationPatchApplyResult {
    pub session_id: i64,
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
    pub content_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptimizationPatchLlmResponse {
    summary: String,
    patches: Vec<OptimizationPatch>,
}

pub fn optimization_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
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
        let replacement = match patch.operation.as_str() {
            "replace" => patch.new_text.clone(),
            "delete" => String::new(),
            "insert_before" => format!("{}{}", patch.new_text, patch.old_text),
            "insert_after" => format!("{}{}", patch.old_text, patch.new_text),
            _ => {
                return Err(AppError::InvalidArgument(format!(
                    "补丁 {} operation 非法：{}",
                    patch.patch_id, patch.operation
                )));
            }
        };
        next_content = next_content.replacen(&patch.old_text, &replacement, 1);
    }
    Ok(next_content)
}

pub(super) fn parse_content_optimization_patch_response(
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
    base_content_hash: &str,
    chapter_content: &str,
    raw: &str,
) -> Result<OptimizationPatchProposal, AppError> {
    let parsed: OptimizationPatchLlmResponse = serde_json::from_str(raw.trim()).map_err(|e| {
        AppError::InvalidArgument(format!(
            "content_optimization_patch 必须返回严格 JSON 对象: {e}"
        ))
    })?;
    let summary = parsed.summary.trim();
    if summary.is_empty() {
        return Err(AppError::InvalidArgument(
            "content_optimization_patch.summary 不能为空".to_string(),
        ));
    }
    if !parsed.patches.is_empty() {
        validate_optimization_patches(chapter_content, &parsed.patches)?;
    }
    Ok(OptimizationPatchProposal {
        session_id,
        project_id,
        chapter_id,
        base_content_hash: base_content_hash.to_string(),
        summary: summary.to_string(),
        patches: parsed
            .patches
            .into_iter()
            .map(normalize_optimization_patch)
            .collect(),
    })
}

fn normalize_optimization_patch(mut patch: OptimizationPatch) -> OptimizationPatch {
    patch.patch_id = patch.patch_id.trim().to_string();
    patch.issue_id = patch
        .issue_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    patch.operation = patch.operation.trim().to_string();
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

fn validate_optimization_patches(
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
    let mut patch_ids = std::collections::HashSet::new();
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
        if !matches!(
            patch.operation.as_str(),
            "replace" | "delete" | "insert_before" | "insert_after"
        ) {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} operation 必须是 replace、delete、insert_before 或 insert_after",
                patch.patch_id
            )));
        }
        if patch.old_text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} old_text 不能为空",
                patch.patch_id
            )));
        }
        if patch.operation == "delete" && !patch.new_text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} operation=delete 时 new_text 必须为空字符串",
                patch.patch_id
            )));
        }
        if patch.operation != "delete" && patch.new_text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "补丁 {} operation={} 时 new_text 不能为空",
                patch.patch_id, patch.operation
            )));
        }
        if patch.operation == "replace" && patch.old_text == patch.new_text {
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
        if patch.operation != "delete" && new_chars > OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS {
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
