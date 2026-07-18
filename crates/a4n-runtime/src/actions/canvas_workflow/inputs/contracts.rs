pub(in crate::actions::canvas_workflow) struct CanvasRenderedPrompt {
    pub(in crate::actions::canvas_workflow) prompt_kind: String,
    pub(in crate::actions::canvas_workflow) prompt_text: String,
    pub(in crate::actions::canvas_workflow) canvas_text_asset_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) source_node_ids: Vec<i64>,
}

#[derive(Debug)]
pub(in crate::actions::canvas_workflow) struct CanvasLlmTransformInput {
    pub(in crate::actions::canvas_workflow) system_prompt: String,
    pub(in crate::actions::canvas_workflow) user_prompt: String,
    pub(in crate::actions::canvas_workflow) instruction: String,
    pub(in crate::actions::canvas_workflow) source_context: String,
    pub(in crate::actions::canvas_workflow) output_kind: String,
    pub(in crate::actions::canvas_workflow) temperature: Option<f32>,
    pub(in crate::actions::canvas_workflow) max_tokens: Option<usize>,
    pub(in crate::actions::canvas_workflow) config_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) canvas_text_asset_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) source_node_ids: Vec<i64>,
}

#[derive(Debug)]
pub(in crate::actions::canvas_workflow) struct CanvasExportVideoInput {
    pub(in crate::actions::canvas_workflow) video_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) source_node_ids: Vec<i64>,
}

#[derive(Debug)]
pub(in crate::actions::canvas_workflow) struct CanvasOutputValue {
    pub(in crate::actions::canvas_workflow) text: Option<String>,
    pub(in crate::actions::canvas_workflow) generated_image_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) generated_video_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) media_asset_id: Option<i64>,
    pub(in crate::actions::canvas_workflow) source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::actions::canvas_workflow) enum GenerateVideoInputMode {
    TextOnly,
    FirstFrame,
    FirstEndFrame,
    ReferenceImages,
}
