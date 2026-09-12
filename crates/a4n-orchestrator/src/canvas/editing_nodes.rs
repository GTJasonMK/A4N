mod contracts;
mod execution;
mod input;

pub(crate) use contracts::CanvasEditingNodeExecutionInput;
#[cfg(test)]
pub(crate) use contracts::{CanvasAssembleVideoNodeInput, CanvasEditingNodeInput};
#[cfg(test)]
pub(crate) use input::build_canvas_editing_node_input_for_preview;

#[cfg(test)]
mod tests;
