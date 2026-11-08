use a4n_domain::source_spans::source_spans_for_content;
use a4n_error::AppError;
use serde::Deserialize;
use serde_json::Value;

use crate::source_markers::SourceMarkerMap;
use crate::structured_llm::parse_strict_json_response;

use super::contracts::{OptimizationPatch, OptimizationPatchProposal};
use super::validation::{normalize_optimization_patch, validate_optimization_patches};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct OptimizationPatchLlmResponse {
    summary: String,
    patches: Vec<OptimizationPatch>,
}

pub(super) fn output_schema() -> Value {
    a4n_domain::json_contract::typed_output_schema::<OptimizationPatchLlmResponse>()
}

#[cfg(test)]
#[test]
fn model_contract_matrix() {
    a4n_test_support::model_contract::emit_schema("content_optimization_patch", output_schema());
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
    let patches = if parsed.patches.is_empty() {
        parsed.patches
    } else {
        let spans = source_spans_for_content(chapter_id, 1, chapter_content);
        let markers = SourceMarkerMap::from_source_spans(&spans)?;
        let patches = parsed
            .patches
            .into_iter()
            .map(|mut patch| {
                patch.old_text = markers.unmark_text(&patch.old_text);
                patch.new_text = markers.unmark_text(&patch.new_text);
                Ok(patch)
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        validate_optimization_patches(chapter_content, &patches)?;
        patches
    };
    Ok(OptimizationPatchProposal {
        session_id,
        project_id,
        chapter_id,
        base_content_hash: base_content_hash.to_string(),
        summary: summary.to_string(),
        patches: patches
            .into_iter()
            .map(normalize_optimization_patch)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn optimization_patch_text_drops_known_source_markers() {
        let content = "林岚推开门。周衡留在门外。";
        let spans = source_spans_for_content(3, 1, content);
        let markers = SourceMarkerMap::from_source_spans(&spans).unwrap();
        let first_marker = markers.marker_for(&spans[0].id).unwrap();
        let proposal = parse_content_optimization_patch_value(
            1,
            2,
            3,
            "content-hash",
            content,
            &json!({
                "summary": "补充可见动作",
                "patches": [{
                    "patch_id": "patch-1",
                    "issue_id": null,
                    "operation": "replace",
                    "old_text": format!("{first_marker} {}", spans[0].text),
                    "new_text": format!("{first_marker} 林岚缓缓推开门。"),
                    "reason": "让动作更清楚",
                    "risk": "low"
                }]
            }),
        )
        .unwrap();

        assert_eq!(proposal.patches[0].old_text, spans[0].text);
        assert_eq!(proposal.patches[0].new_text, "林岚缓缓推开门。");
    }
}
