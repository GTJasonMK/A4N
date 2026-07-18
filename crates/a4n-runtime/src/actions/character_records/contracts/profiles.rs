use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCountsSummary {
    pub explicit: usize,
    pub implicit: usize,
    pub social: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtagonistProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub base_attributes: Value,
    pub current_state: Value,
    pub character_arc: Option<Value>,
    pub voice_fingerprint: Option<Value>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub last_synced_chapter: i32,
    pub attribute_counts: AttributeCountsSummary,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProfileInput {
    pub project_id: i64,
    pub name: String,
    pub base_attributes: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListProfilesInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProfileResultSummary {
    pub deleted_profile_rag: usize,
    pub deleted_state_rag: usize,
    pub deleted_attribute_event_rag: usize,
    pub deleted_profile_id: i64,
    pub deleted_portrait_files: usize,
}
