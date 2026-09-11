//! Pure object-profile synchronization values.

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ObjectProfileEvidenceRecordInput {
    pub normalized_name: String,
    pub text_anchor_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct ObjectProfileRecordInput {
    pub name: String,
    pub normalized_name: String,
    pub aliases_json: Value,
    pub object_type: String,
    pub importance: String,
    pub visual_description: String,
    pub material_and_state: String,
    pub story_usage: String,
    pub visual_usage: String,
    pub generation_usage: String,
    pub location_context: Option<String>,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub evidence_json: Value,
    pub image_prompt_seed: String,
}
