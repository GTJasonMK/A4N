use a4n_error::AppError;
use serde::Deserialize;
use serde_json::Value;

use crate::structured_llm::parse_strict_json_response;

use super::contracts::{OptimizationPatch, OptimizationPatchProposal};
use super::validation::{normalize_optimization_patch, validate_optimization_patches};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OptimizationPatchLlmResponse {
    summary: String,
    patches: Vec<OptimizationPatch>,
}

pub(in crate::optimize) fn parse_content_optimization_patch_response(
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
    base_content_hash: &str,
    chapter_content: &str,
    raw: &str,
) -> Result<OptimizationPatchProposal, AppError> {
    let value = parse_strict_json_response(raw, "content_optimization_patch")?;
    parse_content_optimization_patch_value(
        session_id,
        project_id,
        chapter_id,
        base_content_hash,
        chapter_content,
        &value,
    )
}

fn parse_content_optimization_patch_value(
    session_id: i64,
    project_id: i64,
    chapter_id: i64,
    base_content_hash: &str,
    chapter_content: &str,
    value: &Value,
) -> Result<OptimizationPatchProposal, AppError> {
    let parsed: OptimizationPatchLlmResponse =
        serde_json::from_value(value.clone()).map_err(|e| {
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
