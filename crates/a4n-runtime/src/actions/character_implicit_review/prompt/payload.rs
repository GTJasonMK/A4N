use a4n_prompt::request::build_json_task_user_message;
use serde_json::json;

pub(in crate::actions::character_implicit_review::prompt) fn implicit_update_payload(
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

pub(in crate::actions::character_implicit_review::prompt) fn build_implicit_update_prompt_from_payload(
    payload: serde_json::Value,
) -> Result<String, String> {
    build_json_task_user_message(
        "判断隐性属性是否需要更新",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|e| format!("隐性属性复审上下文 JSON 序列化失败: {e}"))
}
