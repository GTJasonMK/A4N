use super::{
    chapters::{read_current_chapter_content_anchors, select_object_discovery_chapters},
    prompt::{
        build_object_discovery_manifest_user_prompt, existing_object_profile_for_prompt,
        object_discovery_chapter_context, object_discovery_stable_context,
    },
};
use crate::{
    actions::{
        app_error_message,
        llm_request::{
            ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
            context_aware_system_user_json_llm_request,
        },
        object_discovery::{
            types::{ChapterAnchorMode, ObjectDiscoveryRequestContext},
            workflow::object_discovery_business_key,
        },
    },
    state::AppState,
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::json;
use std::collections::HashMap;

pub(in crate::actions::object_discovery) async fn build_object_discovery_request(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    anchor_mode: ChapterAnchorMode,
) -> Result<ObjectDiscoveryRequestContext, String> {
    let chapters = select_object_discovery_chapters(state, project_id, chapter_number).await?;
    let existing_profiles = state
        .orchestrator
        .object_profiles_by_project(project_id)
        .await
        .map_err(app_error_message)?;
    let existing_prompt_profiles = existing_profiles
        .iter()
        .map(existing_object_profile_for_prompt)
        .collect::<Vec<_>>();
    let chapter_data = chapters
        .iter()
        .map(object_discovery_chapter_context)
        .collect::<Vec<_>>();
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
    let system_prompt = state
        .render_prompt("object_discovery", &HashMap::new(), Some(project_id))
        .await
        .map_err(|err| err.to_string())?;
    let stable_context = object_discovery_stable_context(
        &existing_prompt_profiles,
        &chapter_data,
        &chapters,
        &text_anchors,
    );
    let user_prompt = build_object_discovery_manifest_user_prompt()?;
    let chapter_ids = chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let text_anchor_ids = text_anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<Vec<_>>();
    Ok(ObjectDiscoveryRequestContext {
        chapters,
        text_anchors,
        request: context_aware_system_user_json_llm_request(
            system_prompt,
            ContextAwareJsonLlmRequestInput {
                project_id,
                workflow_run_id: None,
                workflow: "object_discovery".to_string(),
                step_key: "object_discovery.parsed".to_string(),
                scope_kind: "profile_sync".to_string(),
                pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
                source_refs_json: Some(json!({
                    "chapter_number": chapter_number,
                    "chapter_ids": chapter_ids,
                    "text_anchor_ids": text_anchor_ids
                })),
                config_json: json!({
                    "ledger_version": "object_discovery_context_v1",
                    "prompt_name": "object_discovery"
                }),
                stable_context_json: stable_context,
                dynamic_user_prompt: user_prompt,
                temperature: Some(0.2),
                max_tokens_profile: llm_token_profiles::CHARACTER_DISCOVERY.to_string(),
                call_key: Some(object_discovery_business_key(project_id, chapter_number)),
            },
        )?,
    })
}
