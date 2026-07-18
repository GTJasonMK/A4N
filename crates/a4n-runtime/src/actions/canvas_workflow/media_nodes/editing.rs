mod extract;
mod trim;

pub(in crate::actions::canvas_workflow) use extract::run_canvas_extract_frame_node;
pub(in crate::actions::canvas_workflow) use trim::run_canvas_trim_video_node;
