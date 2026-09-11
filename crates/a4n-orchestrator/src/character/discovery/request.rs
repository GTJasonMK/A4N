use std::collections::HashMap;

use a4n_db_models::models::{
    chapter::chapter, character::protagonist_profile, project::text_anchor,
};
use a4n_domain::llm::llm_token_profiles;
use a4n_domain::source_spans::text_anchor_prompt_values;
use a4n_error::AppError;
use a4n_llm::{
    context::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
        context_aware_system_user_json_llm_request,
    },
    provider::ChatRequest,
};
use a4n_prompt::{rendering::PromptRenderer, request::build_json_task_user_message};
use serde_json::{Value, json};

use super::{
    CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_KIND, context::character_llm_chapter_context,
    identity::existing_profile_for_prompt,
};

pub(super) async fn build_character_discovery_request(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    existing_profiles: &[protagonist_profile::Model],
) -> Result<ChatRequest, AppError> {
    let existing_prompt_profiles = existing_profiles
        .iter()
        .map(existing_profile_for_prompt)
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::InvalidArgument)?;
    let chapter_data = chapters
        .iter()
        .map(character_llm_chapter_context)
        .collect::<Vec<_>>();
    let system_prompt = prompt_renderer
        .render_prompt(CHARACTER_DISCOVERY_KIND, &HashMap::new(), Some(project_id))
        .await?;
    let stable_context = character_discovery_stable_context(
        &existing_prompt_profiles,
        &chapter_data,
        chapters,
        text_anchors,
    );
    let user_prompt = build_character_discovery_manifest_user_prompt()?;
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
            workflow: CHARACTER_DISCOVERY_KIND.to_string(),
            step_key: CHARACTER_DISCOVERY_CALL_KEY.to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "chapter_number": chapter_number,
                "chapter_ids": chapter_ids,
                "text_anchor_ids": text_anchor_ids,
            })),
            config_json: json!({
                "ledger_version": "character_discovery_context_v1",
                "prompt_name": CHARACTER_DISCOVERY_KIND,
            }),
            stable_context_json: stable_context,
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.2),
            max_tokens_profile: llm_token_profiles::CHARACTER_DISCOVERY.to_string(),
            call_key: Some(CHARACTER_DISCOVERY_CALL_KEY.to_string()),
        },
    )
    .map_err(AppError::InvalidArgument)
}

pub(super) fn build_character_discovery_manifest_user_prompt() -> Result<String, AppError> {
    let payload = character_discovery_payload(json!({
        "context_refs": {
            "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}")
        }
    }));
    build_character_discovery_prompt_from_payload(payload)
}

pub(super) fn character_discovery_stable_context(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Value {
    json!({
        "existing_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors),
    })
}

fn character_discovery_payload(input: Value) -> Value {
    json!({
        "task": {
            "name": CHARACTER_DISCOVERY_KIND,
            "instruction": "从本次输入章节中识别真实人物角色；已有角色档案只用于同一人物判定，不要求模型自行过滤已存在角色"
        },
        "input": input,
        "output_contract": {
            "format": "json_object",
            "required_root_field": "characters",
            "character_scope": "返回本次输入章节中识别出的角色。已有角色也可以返回，后端会归入 existing，不会重复创建。",
            "characters": "array",
            "character_required_fields": [
                "name", "aliases", "importance", "role", "explicit_attributes",
                "implicit_attributes", "social_attributes", "evidence_anchor_id"
            ],
            "system_derived_fields": [
                "appearance", "background", "personality", "first_seen_chapter",
                "latest_seen_chapter", "evidence"
            ],
            "importance_values": ["primary", "secondary", "minor"],
            "chapter_scope": "后端会根据 evidence_anchor_id 所在章节推导 first_seen_chapter 和 latest_seen_chapter；不要输出这两个字段",
            "strict_rules": [
                "只返回可解析的完整 JSON 对象，不要 Markdown 或解释文字",
                "字段要短，属性只保留身份、关系、关键状态等最能区分角色的信息",
                "不要输出 evidence 或 evidence_span_id 字段；必须输出 evidence_anchor_id，并且 ID 必须来自稳定上下文包的 text_anchors[].id",
                "不要输出 first_seen_chapter 或 latest_seen_chapter；系统会从 evidence_anchor_id 推导",
                "不要输出 appearance、background、personality；系统会从三类属性对象中提炼摘要",
                "evidence_anchor_id 只选择最能证明角色首次出场或身份的一个 text_anchor；不要自己抄写原文，不要输出省略号证据",
                "如角色很多，优先保留有姓名、有独立行为、后续可能复用的角色"
            ]
        }
    })
}

fn build_character_discovery_prompt_from_payload(payload: Value) -> Result<String, AppError> {
    build_json_task_user_message(
        "执行角色发现",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|err| {
        AppError::Internal(anyhow::anyhow!("角色发现上下文序列化失败: {err}"))
    })
}
