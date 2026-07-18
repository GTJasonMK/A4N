use super::super::{
    CharacterConflictChapterScope, CharacterConflictRequestContext,
    context::{
        attribute_event_context, character_conflict_allowed_source_ids,
        character_conflict_chapter_context, character_state_context,
        ensure_character_conflict_context_size, ensure_character_conflict_has_comparison_context,
    },
};
use crate::{
    AppState,
    actions::llm_request::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT,
        context_aware_system_user_llm_request,
    },
};
use a4n_core::{config::app_config::llm_token_profiles, error::AppError};
use a4n_prompt::rendering::PromptRenderer;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::json;
use std::collections::HashMap;

pub(super) async fn build_character_conflict_request_unchecked(
    state: &AppState,
    project_id: i64,
    name: &str,
    chapter_scope: CharacterConflictChapterScope,
) -> Result<CharacterConflictRequestContext, AppError> {
    let profile = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .find(|profile| profile.name == name)
        .ok_or_else(|| {
            AppError::NotFound(format!("角色不存在: project_id={project_id}, name={name}"))
        })?;
    let context_rows = state
        .orchestrator
        .character_conflict_context_rows(
            project_id,
            profile.id,
            chapter_scope.start_chapter,
            chapter_scope.end_chapter,
        )
        .await?;

    let role_attr = json!({
        "source_id": "profile",
        "base_attributes": profile.base_attributes,
        "current_state": profile.current_state,
    });
    let mut chapter_context = Vec::new();
    for chapter in &context_rows.chapters {
        if let Some(context) = character_conflict_chapter_context(chapter)? {
            chapter_context.push(context);
        }
    }
    let state_timeline = context_rows
        .state_timeline
        .iter()
        .map(character_state_context)
        .collect::<Vec<_>>();
    let attribute_events = context_rows
        .attribute_events
        .iter()
        .map(attribute_event_context)
        .collect::<Vec<_>>();
    ensure_character_conflict_has_comparison_context(
        &chapter_context,
        &state_timeline,
        &attribute_events,
    )?;
    ensure_character_conflict_context_size(
        &role_attr,
        &chapter_context,
        &state_timeline,
        &attribute_events,
    )?;
    let source_ids = character_conflict_allowed_source_ids(
        &role_attr,
        &chapter_context,
        &state_timeline,
        &attribute_events,
    );
    let chapter_context_count = chapter_context.len();
    let state_timeline_count = state_timeline.len();
    let attribute_event_count = attribute_events.len();
    let stable_context = json!({
        "chapter_scope": chapter_scope.to_context_value(),
        "character_profile": role_attr,
        "chapter_context": chapter_context,
        "character_state_timeline": state_timeline,
        "attribute_events": attribute_events,
    });
    let context_value = json!({
        "task": {
            "name": "character_conflict_check",
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
            "source_id_rule": "expected_source_id 和 actual_source_id 必须从 input 中已有 source_id 选择，分别指向被期望事实和实际冲突事实；不要自造 source_id",
            "severity_values": ["minor", "moderate", "major"],
        },
    });
    let system_prompt = state
        .render_prompt(
            "character_conflict_check",
            &HashMap::new(),
            Some(project_id),
        )
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let user_prompt = build_json_task_user_message(
        "执行角色冲突检查",
        "只返回 output_contract 要求的严格 JSON 数组，不要输出 Markdown、代码块、解释文字或额外字段",
        &context_value,
    )?;
    let call_key = format!("character_conflict:{}:{}", profile.id, name);
    Ok(CharacterConflictRequestContext {
        request: context_aware_system_user_llm_request(
            system_prompt,
            ContextAwareJsonLlmRequestInput {
                project_id,
                workflow_run_id: None,
                workflow: "character_conflict_check".to_string(),
                step_key: "conflict_detection".to_string(),
                scope_kind: "project".to_string(),
                pack_kind: PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT.to_string(),
                source_refs_json: Some(json!({
                    "profile_id": profile.id,
                    "profile_name": &profile.name,
                    "chapter_scope": chapter_scope.to_context_value(),
                    "chapter_context_count": chapter_context_count,
                    "state_timeline_count": state_timeline_count,
                    "attribute_event_count": attribute_event_count,
                })),
                config_json: json!({
                    "ledger_version": "character_conflict_context_v1",
                    "prompt_name": "character_conflict_check",
                }),
                stable_context_json: stable_context,
                dynamic_user_prompt: user_prompt,
                temperature: Some(0.3),
                max_tokens_profile: llm_token_profiles::JSON_STANDARD.to_string(),
                call_key: Some(call_key.clone()),
            },
        )
        .map_err(AppError::InvalidArgument)?,
        source_ids,
        call_key,
    })
}
