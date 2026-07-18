use serde_json::Value;

#[derive(Debug, Clone)]
pub struct InsertBehaviorRecordInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub behavior_type: String,
    pub behavior_description: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification_results: Value,
    pub related_characters: Option<Value>,
    pub impact_analysis: Option<String>,
}
