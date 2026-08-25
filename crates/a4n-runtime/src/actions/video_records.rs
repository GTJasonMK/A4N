mod assembly;
mod completion;
mod deletion;
mod imports;
mod paths;
mod playback;
mod probe;
mod records;

pub use a4n_api_contract::video::UpdateGeneratedVideoInput;
pub use a4n_api_contract::video::VideoPlaybackData;
pub(crate) use assembly::recover_expired_assembly_caches;
pub(in crate::actions::video_records) use completion::{
    complete_video_record, enqueue_unadopted_video_file_cleanup,
};
pub(crate) use imports::recover_expired_uploaded_video_sources;
pub(in crate::actions::video_records) use paths::video_record_output_path;
pub(in crate::actions::video_records) use probe::probe_video_properties;

pub use assembly::{
    ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
    AssembledChapterVideoPreview, AssemblyPushResult, PushAssembledChapterVideoInput,
    UpdateAssemblyPushStatusInput, apply_assembly_postprocess_edit, assemble_chapter_videos,
    load_cached_assembled_chapter_video_preview, push_assembled_chapter_video,
    update_assembly_push_status,
};
pub use deletion::delete_video;
pub(in crate::actions::video_records) use deletion::{
    append_video_record_cleanup_error, delete_video_record_cleanup_error,
};
pub(crate) use imports::import_canvas_video;
pub use imports::{
    UploadAssemblyInsertVideoInput, UploadChapterVideoInput, UploadReferenceMediaVideoInput,
    trim_video, upload_assembly_insert_video, upload_chapter_video, upload_reference_media_video,
};
pub(crate) use imports::{assemble_canvas_videos, extract_canvas_video_frame};
use playback::MAX_INLINE_PLAYBACK_VIDEO_BYTES;
pub use playback::{load_video_playback_data, load_video_scrub_playback_data};
pub use records::{list_videos_by_chapter, select_segment_video, update_video};

#[cfg(test)]
mod tests;
