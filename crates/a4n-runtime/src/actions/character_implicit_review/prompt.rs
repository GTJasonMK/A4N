use super::types::ImplicitAttributeReviewSummary;
use crate::actions::llm_request::PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT;
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::request::build_json_task_user_message;
use serde_json::json;
use std::collections::HashMap;

pub(crate) const IMPLICIT_UPDATE_PROMPT_NAME: &str = "implicit_update";

pub(crate) fn render_implicit_update_system_prompt(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
) -> Result<String, String> {
    prompt_engine
        .render(
            IMPLICIT_UPDATE_PROMPT_NAME,
            &HashMap::new(),
            Some(project_id),
        )
        .map_err(|e| format!("渲染 {IMPLICIT_UPDATE_PROMPT_NAME} 提示词失败: {e}"))
}

pub(crate) fn build_implicit_update_manifest_user_prompt(
    profile_name: &str,
    attribute_key: &str,
) -> Result<String, String> {
    let payload = implicit_update_payload(
        profile_name,
        attribute_key,
        json!({
            "context_refs": {
                "attribute_key": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.attribute_key"),
                "current_value": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.current_value"),
                "review_window": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.review_window"),
                "statistics": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.statistics"),
                "evidence": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.evidence")
            }
        }),
    );
    build_implicit_update_prompt_from_payload(payload)
}

pub(crate) fn implicit_update_stable_context(
    profile_name: &str,
    attribute_key: &str,
    current_value: &serde_json::Value,
    stats: &ImplicitAttributeReviewSummary,
    threshold: usize,
    window_chapters: usize,
) -> serde_json::Value {
    json!({
        "profile_name": profile_name,
        "attribute_key": attribute_key,
        "current_value": current_value,
        "review_window": {
            "window_chapters": window_chapters,
            "non_conform_threshold": threshold,
        },
        "statistics": {
            "total": stats.total,
            "conform_count": stats.conform_count,
            "non_conform_count": stats.non_conform_count,
            "conform_rate": stats.conform_rate,
            "threshold_reached": stats.threshold_reached,
        },
        "evidence": implicit_update_prompt_evidence(stats),
    })
}

#[cfg(test)]
fn build_implicit_update_user_prompt(
    profile_name: &str,
    attribute_key: &str,
    current_value: &serde_json::Value,
    stats: &ImplicitAttributeReviewSummary,
    threshold: usize,
    window_chapters: usize,
) -> Result<String, String> {
    let payload = implicit_update_payload(
        profile_name,
        attribute_key,
        implicit_update_stable_context(
            profile_name,
            attribute_key,
            current_value,
            stats,
            threshold,
            window_chapters,
        ),
    );
    build_implicit_update_prompt_from_payload(payload)
}

fn implicit_update_payload(
    profile_name: &str,
    attribute_key: &str,
    input: serde_json::Value,
) -> serde_json::Value {
    json!({
        "task": {
            "name": "implicit_attribute_update_review",
            "instruction": "判断隐性属性是否需要更新；只根据输入 evidence 和统计结果做 keep/modify/delete 决策"
        },
        "character": {
            "profile_name": profile_name
        },
        "target_attribute": {
            "key": attribute_key
        },
        "input": input,
        "hard_rules": [
            "supporting_behavior_ids 只能从 evidence[].behavior_id 中选择，不能自造 ID",
            "输入 evidence 非空时 supporting_behavior_ids 至少包含 1 个 ID",
            "keep/delete 的 new_value 必须为 null",
            "modify 的 new_value 必须非 null，且应概括 evidence 中一致的新行为模式",
            "不要输出 evidence_summary，系统会根据 supporting_behavior_ids 回填证据摘要"
        ],
        "output_contract": {
            "format": "json_object",
            "required_fields": {
                "decision": "字符串，只能是 keep / modify / delete",
                "reasoning": "字符串，100-200 字，引用章节号、行为描述或 behavior_id",
                "new_value": "modify 时为新隐性属性值；keep/delete 时为 null",
                "supporting_behavior_ids": "整数数组，只能来自 evidence[].behavior_id"
            },
            "forbidden": [
                "Markdown 代码块",
                "解释性前后缀",
                "evidence_summary",
                "输入 evidence 外的 behavior_id"
            ]
        }
    })
}

fn build_implicit_update_prompt_from_payload(payload: serde_json::Value) -> Result<String, String> {
    build_json_task_user_message(
        "判断隐性属性是否需要更新",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|e| format!("隐性属性复审上下文 JSON 序列化失败: {e}"))
}

fn implicit_update_prompt_evidence(
    stats: &ImplicitAttributeReviewSummary,
) -> Vec<serde_json::Value> {
    stats
        .evidence
        .iter()
        .map(|item| {
            json!({
                "behavior_id": item.behavior_id,
                "chapter_number": item.chapter_number,
                "behavior_type": item.behavior_type,
                "behavior_description": item.behavior_description,
                "original_text": item.original_text,
                "behavior_tags": item.behavior_tags,
                "classification": item.classification,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::character_implicit_review::types::ImplicitEvidenceSummary;
    use serde_json::json;

    #[test]
    fn update_prompt_serializes_json_fields_without_fallback() {
        let stats = ImplicitAttributeReviewSummary {
            attribute_key: "谨慎".to_string(),
            current_value: json!("遇事先观察"),
            total: 1,
            conform_count: 0,
            non_conform_count: 1,
            conform_rate: 0.0,
            threshold_reached: true,
            evidence: vec![ImplicitEvidenceSummary {
                behavior_id: 1,
                chapter_number: 3,
                behavior_type: "decision".to_string(),
                behavior_description: "主动推进调查".to_string(),
                original_text: "她没有继续等待。".to_string(),
                behavior_tags: json!(["主动"]),
                classification: json!("non-conform"),
                created_at: None,
            }],
        };

        let prompt =
            build_implicit_update_user_prompt("林夏", "谨慎", &json!("遇事先观察"), &stats, 5, 10)
                .expect("valid JSON values should build prompt");

        assert!(prompt.contains(r#""current_value": "遇事先观察""#));
        assert!(prompt.contains(r#""classification": "non-conform""#));
        assert!(prompt.contains(r#""behavior_id": 1"#));
        assert!(!prompt.contains("created_at"));
        assert!(!prompt.contains("你是小说角色档案复审员"));
    }
}
