use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorRecordSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub behavior_description: String,
    pub behavior_type: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification_results: Value,
    pub related_characters: Option<Value>,
    pub impact_analysis: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListBehaviorsInput {
    pub profile_id: i64,
    pub chapter_from: Option<i32>,
    pub chapter_to: Option<i32>,
    pub limit: Option<u64>,
}
