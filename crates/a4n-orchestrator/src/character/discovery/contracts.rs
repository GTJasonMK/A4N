use std::collections::{BTreeSet, HashMap};

use a4n_core::{
    error::AppError,
    models::{chapter::chapter, project::text_anchor},
    structured_progress::{FieldMilestone, ProgressSpan},
};
use a4n_llm::provider::ChatRequest;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub(super) const CHARACTER_DISCOVERY_PROGRESS_TOTAL: u32 = 100;
pub(super) const CHARACTER_DISCOVERY_LLM_SPAN: ProgressSpan = ProgressSpan::new(20, 75);
pub(super) const CHARACTER_DISCOVERY_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("name", "角色名", 1),
    FieldMilestone::new("aliases", "别名", 1),
    FieldMilestone::new("importance", "重要性", 1),
    FieldMilestone::new("role", "角色定位", 1),
    FieldMilestone::new("explicit_attributes", "显性属性", 1),
    FieldMilestone::new("implicit_attributes", "隐性属性", 1),
    FieldMilestone::new("social_attributes", "社会属性", 1),
    FieldMilestone::new("evidence_anchor_id", "证据锚点", 1),
];

pub type CharacterDiscoveryProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy)]
pub struct CharacterDiscoveryProgress<'a> {
    pub reporter: Option<CharacterDiscoveryProgressReporter<'a>>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub active_task_id: Option<&'a str>,
}

impl CharacterDiscoveryProgress<'_> {
    pub(super) fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, CHARACTER_DISCOVERY_PROGRESS_TOTAL, message)?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterDiscoveryInput {
    pub project_id: i64,
    pub chapter_number: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterDiscoveryTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct CharacterDiscoveryRequestPreview {
    pub analyzed_chapter_count: usize,
    pub request: ChatRequest,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

#[derive(Debug)]
pub(super) struct RequiredCharacterDiscoveryInput {
    pub(super) project_id: i64,
    pub(super) chapter_number: Option<i32>,
}

pub(super) struct CharacterDiscoveryRequestContext {
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) text_anchors: Vec<text_anchor::Model>,
    pub(super) existing_keys: BTreeSet<String>,
    pub(super) existing_key_profiles: HashMap<String, i64>,
    pub(super) request: ChatRequest,
}

pub(super) struct CharacterDiscoveryCallResult {
    pub(super) discovered: Vec<Value>,
    pub(super) response_content: String,
    pub(super) llm_request: ChatRequest,
    pub(super) analyzed_chapters: BTreeSet<i32>,
}

pub(super) struct CharacterDiscoveryCallCheckpoint {
    pub(super) response_content: String,
    pub(super) parsed_output: Value,
}
