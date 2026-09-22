use a4n_domain::prompt_language::PromptOutputLanguage;
use serde_json::Value;

#[derive(Clone, Copy)]
pub(in crate::video) struct PromptWorkflowRowsContext<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: &'a str,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: Option<&'a Value>,
    pub source_prompt_type: Option<&'a str>,
    pub video_execution_identity: &'a Value,
}

pub(super) struct PromptWorkflowRowsQuery<'a> {
    pub context: PromptWorkflowRowsContext<'a>,
    pub prompt_output_language: PromptOutputLanguage,
    pub exclude_running: bool,
}

#[derive(Clone, Copy)]
pub(super) struct PromptGenerationContextForWorkflow<'a> {
    pub expected_workflow_mode: &'a str,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: Option<&'a Value>,
    pub source_prompt_type: Option<&'a str>,
    pub video_execution_identity: &'a Value,
    pub prompt_output_language: PromptOutputLanguage,
}
