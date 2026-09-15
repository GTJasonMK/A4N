use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm_context::{ContextAwareJsonLlmRequestInput, context_aware_system_user_llm_request};
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use crate::character::conflict_context_rules::CharacterConflictChapterScope;
use a4n_api_contract::character::CharacterConflictSeverity;

use super::CHARACTER_CONFLICT_KIND;

pub(super) const PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT: &str =
    "character_conflict_source_context";

pub(super) fn character_conflict_call_key(profile_id: i64) -> String {
    format!("{CHARACTER_CONFLICT_KIND}.profile.{profile_id}")
}

pub(super) struct BuildCharacterConflictRequestInput<'a> {
    pub(super) system_prompt: String,
    pub(super) project_id: i64,
    pub(super) profile_id: i64,
    pub(super) profile_name: &'a str,
    pub(super) chapter_scope: CharacterConflictChapterScope,
    pub(super) role_attr: &'a Value,
    pub(super) chapter_context: &'a [Value],
    pub(super) state_timeline: &'a [Value],
    pub(super) attribute_events: &'a [Value],
}

pub(super) fn build_character_conflict_request(
    input: BuildCharacterConflictRequestInput<'_>,
) -> Result<(String, ChatRequest), AppError> {
    let call_key = character_conflict_call_key(input.profile_id);
    let context_value = json!({
        "task": {
            "name": CHARACTER_CONFLICT_KIND,
            "instruction": "检查角色档案、角色状态时间线、属性事件与章节上下文是否存在事实、性格、关系或状态矛盾",
        },
        "input": {
            "context_refs": {
                "chapter_scope": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.chapter_scope"),
                "character_profile": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.character_profile"),
                "chapter_context": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.chapter_context"),
                "character_state_timeline": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.character_state_timeline"),
                "attribute_events": format!("稳定上下文包 {PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT}.attribute_events"),
            },
        },
        "output_contract": {
            "format": "json_array",
            "empty_result": [],
            "item_required_fields": [
                "chapter_number", "field", "expected", "actual",
                "expected_source_id", "actual_source_id", "severity",
            ],
            "source_id_rule": "expected_source_id 和 actual_source_id 必须从稳定上下文包中已有 source_id 选择，分别指向被期望事实和实际冲突事实；不要自造 source_id",
            "severity_values": [
                CharacterConflictSeverity::Minor,
                CharacterConflictSeverity::Moderate,
                CharacterConflictSeverity::Major,
            ],
        },
    });
    let user_prompt = build_json_task_user_message(
        "执行角色冲突检查",
        "只返回 output_contract 要求的严格 JSON 数组，不要输出 Markdown、代码块、解释文字或额外字段",
        &context_value,
    )?;
    let request = context_aware_system_user_llm_request(
        input.system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id: input.project_id,
            workflow_run_id: None,
            workflow: CHARACTER_CONFLICT_KIND.to_string(),
            step_key: call_key.clone(),
            scope_kind: "project".to_string(),
            pack_kind: PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "profile_id": input.profile_id,
                "profile_name": input.profile_name,
                "chapter_scope": input.chapter_scope.to_context_value(),
                "chapter_context_count": input.chapter_context.len(),
                "state_timeline_count": input.state_timeline.len(),
                "attribute_event_count": input.attribute_events.len(),
            })),
            config_json: json!({
                "ledger_version": "character_conflict_context_v1",
                "prompt_name": CHARACTER_CONFLICT_KIND,
            }),
            stable_context_json: json!({
                "chapter_scope": input.chapter_scope.to_context_value(),
                "character_profile": input.role_attr,
                "chapter_context": input.chapter_context,
                "character_state_timeline": input.state_timeline,
                "attribute_events": input.attribute_events,
            }),
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.3),
            call_key: Some(call_key.clone()),
        },
    )
    .map_err(AppError::InvalidArgument)?;
    Ok((call_key, request))
}
