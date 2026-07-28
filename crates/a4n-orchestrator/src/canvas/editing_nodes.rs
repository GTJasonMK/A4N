mod contracts;
mod execution;
mod input;

pub use contracts::{
    CanvasAssembleVideoNodeInput, CanvasAssembleVideosHostInput, CanvasEditingNodeExecutionInput,
    CanvasEditingNodeHostPort, CanvasEditingNodeInput, CanvasExtractFrameHostInput,
    CanvasExtractFrameNodeInput, CanvasGeneratedImageArtifact, CanvasGeneratedVideoArtifact,
    CanvasTrimVideoHostInput, CanvasTrimVideoNodeInput,
};
pub use input::build_canvas_editing_node_input_for_preview;

#[cfg(test)]
mod tests;
