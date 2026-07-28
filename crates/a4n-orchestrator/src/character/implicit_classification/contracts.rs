use serde_json::Value;

#[derive(Debug, Clone)]
pub struct CharacterImplicitClassificationInput {
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub profile_name: String,
    pub changes: Vec<Value>,
    pub behaviors: Vec<Value>,
    pub final_attributes: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterImplicitClassificationOutput {
    pub behaviors: Vec<Value>,
    pub final_attributes: Value,
    pub classification_suggestions: Vec<Value>,
}
