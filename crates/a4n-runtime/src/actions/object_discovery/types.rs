use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::asset::ObjectProfileDraft;

#[derive(Debug, Clone)]
pub(crate) struct ObjectDiscoveryPreviewStep {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) prompt_name: Option<String>,
    pub(crate) state: String,
    pub(crate) note: Option<String>,
    pub(crate) request: ChatRequest,
}

pub(super) struct ObjectDiscoveryRequestContext {
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) text_anchors: Vec<text_anchor::Model>,
    pub(super) request: ChatRequest,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

#[derive(Debug)]
pub(super) struct ParsedObjectProfileDraft {
    pub(super) draft: ObjectProfileDraft,
    pub(super) evidence_anchor_ids: Vec<i64>,
}

pub(super) struct ObjectDiscoveryCallCheckpoint {
    pub(super) response_content: String,
    pub(super) parsed_output: serde_json::Value,
}
