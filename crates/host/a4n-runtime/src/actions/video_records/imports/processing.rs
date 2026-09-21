mod canvas;
mod trim;

pub use trim::trim_video;

pub(super) use canvas::canvas_frame_output_path;
#[cfg(test)]
pub(in crate::actions::video_records) use trim::persist_trimmed_video_output;
