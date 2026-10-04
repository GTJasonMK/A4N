use std::collections::{BTreeSet, HashMap};

use a4n_domain::{chapter::ChapterRecord, text_anchor::TextAnchorRecord};
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::{FieldMilestone, ProgressSpan};
use serde_json::Value;

pub use a4n_character_host_contract::{
    CharacterDiscoveryProgress, CharacterDiscoveryRequestPreview,
};

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

#[derive(Debug, Clone, Copy)]
pub(super) enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

pub(super) struct CharacterDiscoveryRequestContext {
    pub(super) chapters: Vec<ChapterRecord>,
    pub(super) text_anchors: Vec<TextAnchorRecord>,
    pub(super) existing_keys: BTreeSet<String>,
    pub(super) existing_key_profiles: HashMap<String, i64>,
}

pub(super) struct CharacterDiscoveryCallResult {
    pub(super) discovered: Vec<Value>,
    pub(super) response_content: String,
    pub(super) llm_request: ChatRequest,
    pub(super) analyzed_chapters: BTreeSet<i32>,
}
