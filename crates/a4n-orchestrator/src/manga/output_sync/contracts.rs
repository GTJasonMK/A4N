use serde_json::Value;

use super::super::pipeline_prompt::MangaPromptGenerationTrace;

#[derive(Debug)]
pub(in crate::manga) struct MangaPromptDraft {
    pub(in crate::manga) chapter_id: i64,
    pub(in crate::manga) project_id: i64,
    pub(in crate::manga) page_number: i32,
    pub(in crate::manga) panel_number: i32,
    pub(in crate::manga) prompt_type: String,
    pub(in crate::manga) image_prompt: String,
    pub(in crate::manga) dialogue_text: Option<String>,
    pub(in crate::manga) characters_in_frame: Option<Value>,
    pub(in crate::manga) layout: Option<Value>,
    pub(in crate::manga) camera_angle: Option<String>,
    pub(in crate::manga) dialogue_position: Option<String>,
    pub(in crate::manga) status: String,
    pub(in crate::manga) generation_traces: Vec<MangaPromptGenerationTrace>,
}
