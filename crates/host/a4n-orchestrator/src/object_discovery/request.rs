use std::collections::HashMap;

use a4n_db_models::models::{chapter::chapter, project::text_anchor};
use a4n_domain::object_profile::ObjectProfileRecord;
use a4n_error::AppError;
use a4n_llm_context::{
    ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
    context_aware_system_user_json_llm_request,
};
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::{rendering::PromptRenderer, request::build_json_task_user_message};
use serde_json::{Value, json};

use super::{
    OBJECT_DISCOVERY_CALL_KEY, OBJECT_DISCOVERY_KIND,
    context::{existing_object_profile_for_prompt, object_discovery_stable_context},
};

pub(super) async fn build_object_discovery_request(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    existing_profiles: &[ObjectProfileRecord],
) -> Result<ChatRequest, AppError> {
    let existing_prompt_profiles = existing_profiles
        .iter()
        .map(existing_object_profile_for_prompt)
        .collect::<Vec<_>>();
    let system_prompt = prompt_renderer
        .render_prompt(OBJECT_DISCOVERY_KIND, &HashMap::new(), Some(project_id))
        .await?;
    let stable_context =
        object_discovery_stable_context(&existing_prompt_profiles, chapters, text_anchors)?;
    let chapter_ids = chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let text_anchor_ids = text_anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<Vec<_>>();
    context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id,
            workflow_run_id: None,
            workflow: OBJECT_DISCOVERY_KIND.to_string(),
            step_key: OBJECT_DISCOVERY_CALL_KEY.to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "chapter_number": chapter_number,
                "chapter_ids": chapter_ids,
                "text_anchor_ids": text_anchor_ids,
            })),
            config_json: json!({
                "ledger_version": "object_discovery_context_v1",
                "prompt_name": OBJECT_DISCOVERY_KIND,
            }),
            stable_context_json: stable_context,
            dynamic_user_prompt: build_object_discovery_manifest_user_prompt()?,
            temperature: Some(0.2),
            call_key: Some(OBJECT_DISCOVERY_CALL_KEY.to_string()),
        },
    )
    .map_err(AppError::InvalidArgument)
}

pub(super) fn build_object_discovery_manifest_user_prompt() -> Result<String, AppError> {
    build_object_discovery_prompt_from_payload(object_discovery_payload(json!({
        "context_refs": {
            "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}")
        }
    })))
}

fn object_discovery_payload(input: Value) -> Value {
    json!({
        "task": {
            "name": OBJECT_DISCOVERY_KIND,
            "instruction": "从本次输入章节中识别可复用的关键物体档案；已有物体档案只用于同一物体判定",
        },
        "input": input,
        "output_contract": {
            "format": "json_object",
            "required_root_field": "objects",
            "object_scope": "返回本次输入章节中明确出现且可作为视觉素材复用的物体。已有物体也可以返回，后端会更新而非重复创建。",
            "object_type_values": ["prop", "tool", "weapon", "token", "vehicle", "clothing", "location_object", "environment_object", "other"],
            "importance_values": ["primary", "secondary", "minor"],
            "object_fields": ["name", "aliases", "object_type", "importance", "visual_description", "material_and_state", "story_usage", "visual_usage", "generation_usage", "location_context", "evidence_anchor_ids"],
            "object_type_rules": [
                "普通道具、线索物以外的一般物品用 prop，不要输出 item/object/thing",
                "设备、仪器、工具用 tool",
                "刀、枪、剑、斧等武器用 weapon",
                "信物、遗物、关键道具、象征物用 token，不要输出 key_item/relic/artifact",
                "车、船、火车等载具用 vehicle",
                "衣物、制服、配饰用 clothing",
                "门、窗、桌椅、床、货架等固定场景物件用 location_object",
                "碎石、水洼、烟雾、雾气、灯光等环境物件用 environment_object",
                "只有确实无法归类才用 other",
            ],
            "chapter_scope": "后端会根据 evidence_anchor_ids 所在章节推导 first_seen_chapter 和 last_seen_chapter；不要输出这两个字段",
            "strict_rules": [
                "只返回可解析的完整 JSON 对象，不要 Markdown 或解释文字",
                "不要把纯地点、组织、能力、事件或人物当作物体",
                "每个物体对象只允许输出 output_contract 列出的字段，不要添加额外字段",
                "evidence_anchor_ids 必须选择 chapters[].content 中 ⟦id⟧ 标记的 ID，不含括号",
                "evidence_anchor_ids 是核验用选择题，优先选择物体名称、外观或状态短语对应的 text_anchor；不要自己抄写原文，不要输出省略号证据",
                "location_context 只写简短位置语境文本；无法判断时输出 null，不要输出额外锚点 ID",
                "章节范围和图片提示词种子由系统推导，模型不要输出",
            ],
        },
    })
}

fn build_object_discovery_prompt_from_payload(payload: Value) -> Result<String, AppError> {
    build_json_task_user_message(
        "执行物体档案发现",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|error| AppError::Internal(anyhow::anyhow!("物体发现上下文序列化失败: {error}")))
}
