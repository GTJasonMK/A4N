mod character;
mod foreshadowing;
mod rag;
mod value;

use serde_json::Value;

pub use character::character_state_snapshots;
pub use foreshadowing::{foreshadowing_entries, priority_to_importance};
pub use rag::analysis_rag_items;

#[derive(Debug, Clone)]
pub struct CharacterStateSnapshot {
    pub name: String,
    pub state_summary: String,
    pub key_attributes: Value,
    pub emotional_state: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ForeshadowingEntry {
    pub description: String,
    pub original_text: Option<String>,
    pub category: Option<String>,
    pub priority: Option<String>,
    pub related_entities: Option<Value>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct AnalysisRagItem {
    pub id_suffix: String,
    pub data_type: String,
    pub content_kind: String,
    pub text: String,
    pub metadata: Value,
}

#[cfg(test)]
mod tests;
