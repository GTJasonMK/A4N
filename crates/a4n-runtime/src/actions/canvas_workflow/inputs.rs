mod contracts;
mod media;
mod output;
mod rules;
mod sources;
mod text;

#[cfg(test)]
use contracts::{CanvasExportVideoInput, CanvasRenderedPrompt};
pub(super) use contracts::{CanvasLlmTransformInput, CanvasOutputValue};
pub(super) use media::{
    build_canvas_assemble_input_for_run, build_canvas_export_video_input_for_run,
    build_canvas_extract_frame_input_for_run, build_canvas_image_input_for_run,
    build_canvas_trim_input_for_run, build_canvas_video_input_for_run,
};
pub(super) use output::build_canvas_output_value;
pub(super) use text::{
    build_canvas_llm_transform_input_for_run, build_canvas_prompt_artifact_input_for_run,
    first_canvas_text_asset_id,
};

#[cfg(test)]
mod tests;
