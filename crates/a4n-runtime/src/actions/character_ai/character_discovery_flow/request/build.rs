use std::collections::HashMap;

use a4n_core::{config::app_config::llm_token_profiles, models::project::text_anchor};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::AppState;
use crate::actions::{
    app_error_message,
    llm_request::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
        context_aware_system_user_json_llm_request,
    },
};

use super::super::super::{
    character_discovery::{
        build_character_discovery_manifest_user_prompt, character_discovery_stable_context,
        existing_character_keys, existing_profile_for_prompt, profile_character_keys,
    },
    sync_context::character_llm_chapter_context,
};
use super::super::{
    contracts::{ChapterAnchorMode, CharacterDiscoveryRequestContext},
    rules::select_character_discovery_chapters,
    workflow::character_discovery_business_key,
};

pub(in crate::actions::character_ai::character_discovery_flow) async fn build_character_discovery_request(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    anchor_mode: ChapterAnchorMode,
) -> Result<CharacterDiscoveryRequestContext, String> {
    let available_chapters = state
        .orchestrator
        .character_sync_chapters(project_id)
        .await
        .map_err(|err| err.to_string())?;
    let chapters =
        select_character_discovery_chapters(available_chapters, project_id, chapter_number)?;

    let existing_profiles = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    let existing_keys = existing_character_keys(&existing_profiles);
    let existing_key_profiles = existing_profiles
        .iter()
        .flat_map(|profile| {
            profile_character_keys(profile)
                .into_iter()
                .map(|key| (key, profile.id))
        })
        .collect::<HashMap<_, _>>();
    let existing_prompt_profiles: Vec<Value> = existing_profiles
        .iter()
        .map(existing_profile_for_prompt)
        .collect::<Result<_, _>>()?;
    let mut text_anchors = Vec::new();
    for chapter in &chapters {
        let anchors = match anchor_mode {
            ChapterAnchorMode::EnsureCurrent => state
                .orchestrator
                .ensure_chapter_content_anchor_records(project_id, chapter.id)
                .await
                .map_err(app_error_message)?,
            ChapterAnchorMode::RequireExistingCurrent => {
                read_current_chapter_content_anchors(state, project_id, chapter.id).await?
            }
        };
        text_anchors.extend(anchors);
    }
    let chapter_data: Vec<Value> = chapters.iter().map(character_llm_chapter_context).collect();
    let system_prompt = state
        .render_prompt("character_discovery", &HashMap::new(), Some(project_id))
        .await
        .map_err(|e| e.to_string())?;
    let stable_context = character_discovery_stable_context(
        &existing_prompt_profiles,
        &chapter_data,
        &chapters,
        &text_anchors,
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

    Ok(CharacterDiscoveryRequestContext {
        chapters,
        text_anchors,
        existing_keys,
        existing_key_profiles,
        request: context_aware_system_user_json_llm_request(
            system_prompt,
            ContextAwareJsonLlmRequestInput {
                project_id,
                workflow_run_id: None,
                workflow: "character_discovery".to_string(),
                step_key: "character_discovery.parsed".to_string(),
                scope_kind: "profile_sync".to_string(),
                pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
                source_refs_json: Some(json!({
                    "chapter_number": chapter_number,
                    "chapter_ids": chapter_ids,
                    "text_anchor_ids": text_anchor_ids})),
                config_json: json!({
                    "ledger_version": "character_discovery_context_v1",
                    "prompt_name": "character_discovery"}),
                stable_context_json: stable_context,
                dynamic_user_prompt: user_prompt,
                temperature: Some(0.2),
                max_tokens_profile: llm_token_profiles::CHARACTER_DISCOVERY.to_string(),
                call_key: Some(character_discovery_business_key(project_id, chapter_number)),
            },
        )?,
    })
}

async fn read_current_chapter_content_anchors(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<text_anchor::Model>, String> {
    let rows = state
        .orchestrator
        .read_current_chapter_content_anchor_records(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    if rows.anchors.is_empty() {
        return Err(format!(
            "状态无效：当前状态 第 {} 章缺少当前正文文本锚点，不允许操作 预览模型请求",
            rows.chapter.chapter_number
        ));
    }
    if rows
        .anchors
        .iter()
        .any(|anchor| anchor.source_version_hash != rows.source_version_hash)
    {
        return Err(format!(
            "状态无效：当前状态 第 {} 章正文文本锚点已过期，不允许操作 预览模型请求",
            rows.chapter.chapter_number
        ));
    }
    Ok(rows.anchors)
}
