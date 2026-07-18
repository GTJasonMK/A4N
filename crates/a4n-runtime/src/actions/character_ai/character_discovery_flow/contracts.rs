use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::structured_progress::{FieldMilestone, ProgressSpan};
use a4n_llm::provider::ChatRequest;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

pub(super) const CHARACTER_DISCOVERY_PROGRESS_TOTAL: u32 = 100;
pub(super) const CHARACTER_DISCOVERY_LLM_SPAN: ProgressSpan = ProgressSpan::new(20, 75);
pub(super) const CHARACTER_DISCOVERY_CALL_KEY: &str = "character_discovery.llm";
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

pub(super) struct CharacterDiscoveryRequestContext {
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) text_anchors: Vec<text_anchor::Model>,
    pub(super) existing_keys: BTreeSet<String>,
    pub(super) existing_key_profiles: HashMap<String, i64>,
    pub(super) request: ChatRequest,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

pub(super) struct CharacterDiscoveryCallCheckpoint {
    pub(super) response_content: String,
    pub(super) parsed_output: Value,
}
