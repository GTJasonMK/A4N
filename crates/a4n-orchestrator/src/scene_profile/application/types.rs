use crate::scene_profile::SceneSyncInput;
use a4n_core::models::asset::object_profile;
use a4n_core::models::chapter::chapter;
use a4n_core::models::character::protagonist_profile;
use a4n_core::models::project::text_anchor;
use a4n_core::models::scene::scene_profile;
use a4n_core::structured_progress::{ProgressSpan, StructuredJsonProgressSpec};
use a4n_llm::provider::ChatRequest;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const SCENE_PROFILE_CANDIDATES_PROMPT: &str = "scene_profile_candidates";
pub(super) const SCENE_OCCURRENCE_PROMPT: &str = "scene_occurrence_extraction";
pub(super) const SCENE_TRANSITION_PROMPT: &str = "scene_transition_extraction";
pub(super) const SCENE_ENTITY_STATE_PROMPT: &str = "scene_entity_state_extraction";
pub(super) const SCENE_TYPE_RULE: &str = "scene_type 只能是 room/building/street/city/wilderness/route/landmark/other；走廊、通道、楼梯、电梯、道路、小路等连接路径使用 route，不要输出 corridor/hallway/road";
pub(super) const SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE: &str = "scenes 只能包含本章正文中实际出现、进入、停留或发生行动的场景；只被提到、传闻、背景设定、心理想象或未实际出场的地点不要输出；每个输出的 scene_ref 后续必须能生成至少一条 occurrences";
pub(super) const SCENE_OCCURRENCE_COVERAGE_RULE: &str = "必须覆盖 input.previous_stages.scenes[] 中的每一个 scene_ref：每个 scene_ref 至少输出一条 occurrence；只有 input.previous_stages.scenes 为空时才允许返回 occurrences: []";
pub(super) const SCENE_TRANSITION_REFERENCE_RULE: &str = "to_scene_ref 必须来自 input.previous_stages.scenes[].scene_ref 且不得为 null；from_scene_ref 必须来自该数组或为 null；仅知道离开当前场景但目标场景不明确时不要输出该条转场";
pub(super) const SCENE_TRANSITION_MOVEMENT_TYPE_RULE: &str = "movement_type 只能是 enter/leave/travel/chase/teleport/hidden_passage/cutaway；普通场景间移动或换地点使用 travel，不要输出 move";
pub(super) const SCENE_ENTITY_TYPE_RULE: &str = "entity_type 只能是 character/object/event；物体、道具、装备或线索物统一使用 object，不要输出 item/prop/tool";
pub(super) const SCENE_ENTITY_STATE_KEY_RULE: &str = "state_key 只能是 position/possession/condition/action/relation/event_progress；位置用 position，持有关系用 possession，状态/情绪/身体状况用 condition，动作行为用 action，关系用 relation，事件进展用 event_progress";

#[derive(Debug, Clone)]
pub struct SceneProfileSyncPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

pub(super) struct SceneProfileSyncRequestContext {
    pub(super) chapter: chapter::Model,
    pub(super) text_anchors: Vec<text_anchor::Model>,
    pub(super) existing_scene_profiles: Vec<scene_profile::Model>,
    pub(super) character_profiles: Vec<protagonist_profile::Model>,
    pub(super) object_profiles: Vec<object_profile::Model>,
    pub(super) existing_scene_ids: BTreeSet<i64>,
    pub(super) character_ids: BTreeSet<i64>,
    pub(super) object_ids: BTreeSet<i64>,
    pub(super) character_names: BTreeMap<i64, String>,
    pub(super) object_names: BTreeMap<i64, String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

pub(super) struct SceneProfileSyncStageResponses {
    pub(super) scenes: Value,
    pub(super) occurrences: Value,
    pub(super) transitions: Value,
    pub(super) entity_states: Value,
    pub(super) requests: Vec<ChatRequest>,
    pub(super) response_chars: Vec<Value>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SceneProfileStageProgressSpec {
    pub(super) structured: StructuredJsonProgressSpec,
    pub(super) span: ProgressSpan,
}

pub(super) struct SceneProfileStageCallResult {
    pub(super) output: Value,
    pub(super) response_content: String,
}

pub(super) struct SceneProfileStageCheckpoint {
    pub(super) response_content: String,
    pub(super) parsed_output: Value,
}

#[derive(Debug)]
pub(super) struct ParsedSceneProfileSync {
    pub(super) input: SceneSyncInput,
    pub(super) profile_evidence: Vec<SceneProfileEvidenceRefs>,
}

#[derive(Debug)]
pub(super) struct SceneProfileEvidenceRefs {
    pub(super) scene_ref: String,
    pub(super) evidence_anchor_ids: Vec<i64>,
}
