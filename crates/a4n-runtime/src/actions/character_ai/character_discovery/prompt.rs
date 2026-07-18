use serde_json::{Value, json};

use a4n_core::models::chapter::chapter;
use a4n_core::models::project::text_anchor;
use a4n_core::source_spans::text_anchor_prompt_values;
use a4n_prompt::request::build_json_task_user_message;

use crate::actions::llm_request::PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT;

#[cfg(test)]
pub(in crate::actions::character_ai) fn build_character_discovery_user_prompt(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<String, String> {
    let payload = character_discovery_payload(json!({
        "existing_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors)
    }));
    build_character_discovery_prompt_from_payload(payload)
}

pub(in crate::actions::character_ai) fn build_character_discovery_manifest_user_prompt()
-> Result<String, String> {
    let payload = character_discovery_payload(json!({
        "context_refs": {
            "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}")
        }
    }));
    build_character_discovery_prompt_from_payload(payload)
}

pub(in crate::actions::character_ai) fn character_discovery_stable_context(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Value {
    json!({
        "existing_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors)
    })
}

fn character_discovery_payload(input: Value) -> Value {
    json!({
        "task": {
            "name": "character_discovery",
            "instruction": "从本次输入章节中识别真实人物角色；已有角色档案只用于同一人物判定，不要求模型自行过滤已存在角色"
        },
        "input": input,
        "output_contract": {
            "format": "json_object",
            "required_root_field": "characters",
            "character_scope": "返回本次输入章节中识别出的角色。已有角色也可以返回，后端会归入 existing，不会重复创建。",
            "characters": "array",
            "size_limits": {
                "max_characters": 24,
                "aliases_max_items": 4,
                "attribute_object_max_keys_each": 4,
                "role_max_chars": 12,
                "evidence_anchor_id_count": 1
            },
            "character_required_fields": [
                "name",
                "aliases",
                "importance",
                "role",
                "explicit_attributes",
                "implicit_attributes",
                "social_attributes",
                "evidence_anchor_id"
            ],
            "system_derived_fields": [
                "appearance",
                "background",
                "personality",
                "first_seen_chapter",
                "latest_seen_chapter",
                "evidence"
            ],
            "importance_values": ["primary", "secondary", "minor"],
            "chapter_scope": "后端会根据 evidence_anchor_id 所在章节推导 first_seen_chapter 和 latest_seen_chapter；不要输出这两个字段",
            "strict_rules": [
                "只返回可解析的完整 JSON 对象，不要 Markdown 或解释文字",
                "字段要短，属性只保留身份、关系、关键状态等最能区分角色的信息",
                "不要输出 evidence 或 evidence_span_id 字段；必须输出 evidence_anchor_id，并且 ID 必须来自 input.text_anchors[].id",
                "不要输出 first_seen_chapter 或 latest_seen_chapter；系统会从 evidence_anchor_id 推导",
                "不要输出 appearance、background、personality；系统会从三类属性对象中提炼摘要",
                "evidence_anchor_id 只选择最能证明角色首次出场或身份的一个 text_anchor；不要自己抄写原文，不要输出省略号证据",
                "如角色很多，优先保留有姓名、有独立行为、后续可能复用的角色"
            ]
        }
    })
}

fn build_character_discovery_prompt_from_payload(payload: Value) -> Result<String, String> {
    build_json_task_user_message(
        "执行角色发现",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|err| format!("角色发现上下文序列化失败: {err}"))
}
