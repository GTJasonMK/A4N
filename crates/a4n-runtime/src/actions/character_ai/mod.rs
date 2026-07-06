mod character_discovery;
mod character_discovery_flow;
mod implicit_classification;
mod json_utils;
mod snapshot_storage;
mod state_projection;
mod sync_batch;
mod sync_context;
mod sync_implicit;
mod sync_llm;
mod sync_preflight;
mod sync_profile;
mod sync_response;
mod sync_scope;
mod sync_trace;
mod sync_types;
mod sync_validate;
mod sync_write;

use crate::{
    AppState,
    actions::{
        access,
        request_preview::{
            ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
        },
    },
};
use a4n_engine::llm::provider::ChatRequest;
use tokio_util::sync::CancellationToken;

pub(crate) use character_discovery_flow::character_discovery_preview_steps;
pub use character_discovery_flow::discover_character_profiles_for_chapters;
pub use sync_batch::sync_all_character_profiles_to_chapter;
pub(crate) use sync_batch::{
    sync_all_character_profiles_preview_steps, sync_analysis_character_profiles_to_chapter,
};
pub(crate) use sync_preflight::preflight_analysis_character_profile_sync_to_chapter;
pub(crate) use sync_profile::character_sync_preview_steps;
pub use sync_profile::sync_character_profile_from_chapters;
pub use sync_types::CHARACTER_PROFILE_SYNC_STEPS;

#[cfg(test)]
use a4n_core::models::chapter::chapter;
#[cfg(test)]
use a4n_core::models::character::{attribute_event, protagonist_profile};
#[cfg(test)]
use character_discovery::{
    build_character_discovery_user_prompt, discovered_character_base_attributes,
    parse_character_discovery_output,
};
#[cfg(test)]
use character_discovery_flow::{
    select_character_discovery_chapters, validate_discovered_character_chapter_scope,
};
#[cfg(test)]
use implicit_classification::{
    apply_implicit_classification_to_behavior, build_implicit_classification_prompt,
    parse_implicit_classification_output,
};
#[cfg(test)]
use json_utils::attribute_counts;
#[cfg(test)]
use serde_json::{Value, json};
#[cfg(test)]
use snapshot_storage::attribute_events_at_chapter;
#[cfg(test)]
use state_projection::pending_attribute_events;
#[cfg(test)]
use state_projection::{
    apply_attribute_to_state, fold_attribute_event_tuples, project_attribute_changes,
};
#[cfg(test)]
use sync_context::character_llm_chapter_context;
#[cfg(test)]
use sync_llm::ensure_character_sync_response_not_truncated;
#[cfg(test)]
use sync_response::{
    chapter_state_drafts, parse_character_sync_output, validate_unique_deletion_candidates,
};
#[cfg(test)]
use sync_scope::{
    CharacterSyncRange, analysis_profile_sync_skip_reason, analysis_sync_character_names,
    character_names_from_chapter_analysis, character_names_from_discovery_result,
    character_sync_range, character_sync_start_chapter, profile_first_seen_chapter,
    sync_skip_summary, validate_character_sync_target_chapter,
};
#[cfg(test)]
use sync_scope::{CharacterSyncRangeMode, profile_sync_skip_reason};
#[cfg(test)]
use sync_types::CharacterSyncContext;
#[cfg(test)]
use sync_validate::{
    sort_sync_items_by_chapter, validate_contiguous_sync_range,
    validate_sync_evidence_against_chapters, validate_sync_item_chapters,
};
#[cfg(test)]
use sync_write::AttributeEventDraft;
#[cfg(test)]
use sync_write::{SyncWriteStepKind, sync_write_steps};

#[derive(Debug, Clone, Copy)]
pub struct CharacterAiTaskScope {
    pub project_id: i64,
    pub total: u32,
}

pub async fn preview_discover_character_profiles_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<ModelRequestPreview, String> {
    character_ai_preview(
        state,
        "character_discovery",
        "角色发现 LLM 请求预览",
        character_discovery_preview_steps(
            state,
            user_id,
            project_id,
            chapter_number,
            "ready",
            None,
        )
        .await?,
    )
    .await
}

pub async fn preview_sync_character_from_chapters_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    chapter_number: i32,
) -> Result<ModelRequestPreview, String> {
    character_ai_preview(
        state,
        "character_profile_sync",
        format!("{name} 角色同步 LLM 请求预览"),
        character_sync_preview_steps(state, user_id, project_id, name, chapter_number).await?,
    )
    .await
}

pub async fn preview_sync_all_character_profiles_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<ModelRequestPreview, String> {
    character_ai_preview(
        state,
        "character_profile_sync_all",
        "批量角色同步 LLM 请求预览",
        sync_all_character_profiles_preview_steps(state, user_id, project_id, chapter_number)
            .await?,
    )
    .await
}

async fn character_ai_preview(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    source_steps: Vec<CharacterAiPreviewStep>,
) -> Result<ModelRequestPreview, String> {
    chat_status_preview_for_state(
        state,
        workflow,
        title,
        source_steps.into_iter().map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        }),
        "llm_chat",
    )
    .await
}

pub async fn character_discovery_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(state, user_id, project_id, 100).await
}

pub async fn character_profile_sync_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(state, user_id, project_id, CHARACTER_PROFILE_SYNC_STEPS).await
}

pub async fn character_profile_sync_all_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterAiTaskScope, String> {
    character_ai_project_task_scope(state, user_id, project_id, 1).await
}

pub type CharacterAiProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

pub struct CharacterAiProgress<'a> {
    reporter: Option<CharacterAiProgressReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> CharacterAiProgress<'a> {
    pub fn new(
        reporter: Option<CharacterAiProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    pub(crate) fn report(
        &self,
        current: u32,
        total: u32,
        message: &str,
        cancelled_message: &str,
    ) -> Result<(), String> {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(cancelled_message.to_string())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharacterAiPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

impl CharacterAiPreviewStep {
    pub(crate) fn llm(
        key: impl Into<String>,
        title: impl Into<String>,
        prompt_name: &str,
        state: impl Into<String>,
        note: Option<String>,
        request: ChatRequest,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            prompt_name: Some(prompt_name.to_string()),
            state: state.into(),
            note,
            request: Some(request),
        }
    }

    pub(crate) fn status(
        key: impl Into<String>,
        title: impl Into<String>,
        prompt_name: &str,
        state: impl Into<String>,
        note: Option<String>,
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            prompt_name: Some(prompt_name.to_string()),
            state: state.into(),
            note,
            request: None,
        }
    }
}

async fn character_ai_project_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    total: u32,
) -> Result<CharacterAiTaskScope, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(CharacterAiTaskScope { project_id, total })
}

#[cfg(test)]
mod tests;
