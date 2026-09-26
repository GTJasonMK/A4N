use std::collections::HashMap;

use serde_json::Value;

use super::{ChapterVideoSourceContext, PromptRenderer};

pub(super) const VIDEO_CONTEXT_SCOPE: &str = "chapter";
pub(super) const VIDEO_CONTEXT_LEDGER_VERSION: &str = "video_context_ledger_v1";

pub(in crate::video) const VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT: &str =
    "video_previous_output:shooting_script_projection";

pub(in crate::video) struct VideoContextPreviousOutput<'a> {
    pub pack_kind: &'a str,
    pub source_name: &'a str,
    pub parsed_output: &'a Value,
}

pub(in crate::video) struct VideoContextManifestInput<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub project_id: i64,
    pub variables: &'a HashMap<String, Value>,
    pub source: &'a ChapterVideoSourceContext<'a>,
    pub prompt_names: &'a [&'a str],
    pub previous_outputs: &'a [VideoContextPreviousOutput<'a>],
    pub project_context: Option<&'a Value>,
    pub dynamic_user_content: String,
}
