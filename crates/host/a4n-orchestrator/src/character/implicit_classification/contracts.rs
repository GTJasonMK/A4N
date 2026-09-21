use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct CharacterImplicitClassificationInput<'a> {
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub profile_id: i64,
    pub profile_name: &'a str,
    pub changes: &'a [Value],
    pub behaviors: Vec<Value>,
    pub final_attributes: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CharacterImplicitClassificationOutput {
    pub behaviors: Vec<Value>,
    pub final_attributes: Value,
    pub classification_suggestions: Vec<Value>,
}
