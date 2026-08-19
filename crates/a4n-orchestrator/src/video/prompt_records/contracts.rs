use a4n_core::prompt_language::PromptOutputLanguage;
use serde_json::Value;

#[cfg(test)]
pub struct VideoPromptUpdateInput<'a> {
    pub visual_prompt: Option<&'a str>,
    pub motion_prompt: Option<Option<&'a str>>,
    pub negative_prompt: Option<Option<&'a str>>,
    pub duration_secs: Option<i32>,
    pub reference_image_ids_json: Option<Option<Value>>,
}

#[derive(Clone, Copy)]
pub(in crate::video) enum VideoExecutionIdentityMatch<'a> {
    #[cfg(test)]
    NotApplicable,
    Segment(&'a Value),
}

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
    pub video_execution_identity: VideoExecutionIdentityMatch<'a>,
}

pub(super) struct PromptWorkflowRowsQuery<'a> {
    pub context: PromptWorkflowRowsContext<'a>,
    pub prompt_output_language: PromptOutputLanguage,
    pub exclude_running: bool,
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) struct PromptGenerationContext<'a> {
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: Option<&'a Value>,
    pub source_prompt_type: Option<&'a str>,
    pub video_execution_identity: VideoExecutionIdentityMatch<'a>,
    pub prompt_output_language: PromptOutputLanguage,
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
    pub video_execution_identity: VideoExecutionIdentityMatch<'a>,
    pub prompt_output_language: PromptOutputLanguage,
}
