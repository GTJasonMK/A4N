use crate::actions::llm_request::PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT;
use a4n_core::{
    models::{asset::object_profile, chapter::chapter, project::text_anchor},
    source_spans::text_anchor_prompt_values,
};
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

pub(super) fn build_object_discovery_manifest_user_prompt() -> Result<String, String> {
    let payload = object_discovery_payload(json!({"context_refs": {
        "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}")
    }}));
    build_object_discovery_prompt_from_payload(payload)
}

pub(super) fn object_discovery_stable_context(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Value {
    json!({
        "existing_object_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors)
    })
}

pub(super) fn object_discovery_chapter_context(chapter: &chapter::Model) -> Value {
    json!({"chapter_number": chapter.chapter_number, "chapter_title": chapter.title, "content": chapter.content, "analysis_data": chapter.analysis_data})
}

pub(super) fn existing_object_profile_for_prompt(profile: &object_profile::Model) -> Value {
    json!({"name": profile.name, "aliases": profile.aliases_json, "object_type": profile.object_type, "importance": profile.importance, "visual_description": profile.visual_description, "material_and_state": profile.material_and_state, "story_usage": profile.story_usage, "visual_usage": profile.visual_usage, "generation_usage": profile.generation_usage, "location_context": profile.location_context, "first_seen_chapter": profile.first_seen_chapter, "last_seen_chapter": profile.last_seen_chapter})
}

fn object_discovery_payload(input: Value) -> Value {
    json!({
        "task": {"name": "object_discovery", "instruction": "从本次输入章节中识别可复用的关键物体档案；已有物体档案只用于同一物体判定"},
        "input": input,
        "output_contract": {
            "format": "json_object", "required_root_field": "objects",
            "object_scope": "返回本次输入章节中明确出现且可作为视觉素材复用的物体。已有物体也可以返回，后端会更新而非重复创建。",
            "object_type_values": ["prop", "tool", "weapon", "token", "vehicle", "clothing", "location_object", "environment_object", "other"],
            "importance_values": ["primary", "secondary", "minor"],
            "object_fields": ["name", "aliases", "object_type", "importance", "visual_description", "material_and_state", "story_usage", "visual_usage", "generation_usage", "location_context", "evidence_anchor_ids"],
            "object_type_rules": ["普通道具、线索物以外的一般物品用 prop，不要输出 item/object/thing", "设备、仪器、工具用 tool", "刀、枪、剑、斧等武器用 weapon", "信物、遗物、关键道具、象征物用 token，不要输出 key_item/relic/artifact", "车、船、火车等载具用 vehicle", "衣物、制服、配饰用 clothing", "门、窗、桌椅、床、货架等固定场景物件用 location_object", "碎石、水洼、烟雾、雾气、灯光等环境物件用 environment_object", "只有确实无法归类才用 other"],
            "chapter_scope": "后端会根据 evidence_anchor_ids 所在章节推导 first_seen_chapter 和 last_seen_chapter；不要输出这两个字段",
            "strict_rules": ["只返回可解析的完整 JSON 对象，不要 Markdown 或解释文字", "不要把纯地点、组织、能力、事件或人物当作物体", "每个物体对象只允许输出 output_contract 列出的字段，不要添加额外字段", "evidence_anchor_ids 中的每个 ID 必须来自 input.text_anchors[].id", "evidence_anchor_ids 是核验用选择题，优先选择物体名称、外观或状态短语对应的 text_anchor；不要自己抄写原文，不要输出省略号证据", "location_context 只写简短位置语境文本；无法判断时输出 null，不要输出额外锚点 ID", "章节范围和图片提示词种子由系统推导，模型不要输出"]
        }
    })
}

fn build_object_discovery_prompt_from_payload(payload: Value) -> Result<String, String> {
    build_json_task_user_message(
        "执行物体档案发现",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|err| format!("物体发现上下文序列化失败: {err}"))
}
