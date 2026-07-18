mod canvas;
mod metadata;
mod trim;

pub use trim::trim_video;

pub(super) use canvas::{
    canvas_assembly_manifest, canvas_frame_output_path, canvas_frame_timestamp,
    normalize_canvas_frame_kind, run_ffmpeg_extract_frame,
};
pub(in crate::actions::video_records) use metadata::imported_chapter_video_metadata;
#[cfg(test)]
pub(in crate::actions::video_records) use trim::{
    persist_trimmed_video_output, trimmed_video_metadata, validate_video_trim_range,
};
