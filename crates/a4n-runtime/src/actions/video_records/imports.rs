mod canvas;
mod chapter;
mod contracts;
mod prelude;
mod processing;
mod reference;
mod source;

pub use canvas::{assemble_canvas_videos, extract_canvas_video_frame};
pub(crate) use canvas::import_canvas_video;
pub use chapter::upload_chapter_video;
pub(in crate::actions::video_records) use contracts::ImportedVideoSource;
pub use contracts::{
    UploadAssemblyInsertVideoInput, UploadChapterVideoInput, UploadReferenceMediaVideoInput,
};
pub use processing::trim_video;
pub use reference::{upload_assembly_insert_video, upload_reference_media_video};

#[cfg(test)]
pub(in crate::actions::video_records) use processing::persist_trimmed_video_output;
use source::persist_imported_video_file;
pub(crate) use source::recover_expired_uploaded_video_sources;
#[cfg(test)]
pub(in crate::actions::video_records) use source::{
    copy_imported_video_file, expired_uploaded_video_sources, import_video_source_path,
    persist_uploaded_video_source,
};
pub(in crate::actions::video_records) use source::{
    probe_imported_video_properties, uploaded_video_source,
};
