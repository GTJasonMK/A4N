use serde_json::Value;
use std::collections::BTreeMap;

pub const SCENE_TYPES: &[&str] = &[
    "room",
    "building",
    "street",
    "city",
    "wilderness",
    "route",
    "landmark",
    "other",
];
pub const SCENE_TRANSITION_MOVEMENT_TYPES: &[&str] = &[
    "enter",
    "leave",
    "travel",
    "chase",
    "teleport",
    "hidden_passage",
    "cutaway",
];
pub const SCENE_ENTITY_TYPES: &[&str] = &["character", "object", "event"];
pub const SCENE_ENTITY_STATE_KEYS: &[&str] = &[
    "position",
    "possession",
    "condition",
    "action",
    "relation",
    "event_progress",
];

#[derive(Debug, Clone)]
pub struct SceneProfileDraft {
    pub scene_ref: String,
    pub scene_profile_id: Option<i64>,
    pub name: String,
    pub aliases: Vec<String>,
    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,
    pub base_description: String,
    pub visual_description: String,
    pub spatial_layout: String,
    pub atmosphere: String,
    pub entry_points: Value,
    pub rules: Value,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence: Vec<String>,
    pub image_prompt_seed: String,
}

#[derive(Debug, Clone)]
pub struct SceneOccurrenceDraft {
    pub occurrence_ref: String,
    pub scene_ref: String,
    pub occurrence_order: i32,
    pub timeline_position: Option<String>,
    pub summary: String,
    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,
    pub source_anchor_ids: Vec<i64>,
    pub evidence: Vec<String>,
    pub characters: Value,
    pub objects: Value,
    pub key_events: Value,
    pub entry_state: Value,
    pub exit_state: Value,
}

#[derive(Debug, Clone)]
pub struct SceneTransitionDraft {
    pub from_scene_ref: Option<String>,
    pub to_scene_ref: String,
    pub occurrence_ref: Option<String>,
    pub transition_order: i32,
    pub movement_type: String,
    pub route_description: String,
    pub time_cost: Option<String>,
    pub distance_hint: Option<String>,
    pub participants: Value,
    pub object_refs: Value,
    pub source_anchor_ids: Vec<i64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SceneEntityStateDraft {
    pub scene_ref: String,
    pub occurrence_ref: String,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub entity_name_snapshot: String,
    pub state_key: String,
    pub state_before: Value,
    pub state_after: Value,
    pub position_description: Option<String>,
    pub action_description: String,
    pub source_anchor_ids: Vec<i64>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SceneSyncInput {
    pub profiles: Vec<SceneProfileDraft>,
    pub occurrences: Vec<SceneOccurrenceDraft>,
    pub transitions: Vec<SceneTransitionDraft>,
    pub entity_states: Vec<SceneEntityStateDraft>,
}

#[derive(Debug, Clone, Default)]
pub struct SceneSyncResult {
    pub profile_ids: Vec<i64>,
    pub profile_ref_ids: BTreeMap<String, i64>,
    pub occurrence_count: usize,
    pub transition_count: usize,
    pub entity_state_count: usize,
}
