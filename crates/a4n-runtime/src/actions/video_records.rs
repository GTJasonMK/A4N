mod assembly;
mod deletion;
mod imports;
mod playback;
mod records;

pub use a4n_api_contract::video::VideoPlaybackData;
pub use a4n_orchestrator::video::UpdateGeneratedVideoInput;
pub(crate) use assembly::recover_expired_assembly_caches;
pub(crate) use imports::recover_expired_uploaded_video_sources;

pub use assembly::{
    ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
    AssembledChapterVideoPreview, AssemblyPushResult, PushAssembledChapterVideoInput,
    UpdateAssemblyPushStatusInput, apply_assembly_postprocess_edit, assemble_chapter_videos,
    load_cached_assembled_chapter_video_preview, push_assembled_chapter_video,
    update_assembly_push_status,
};
pub use deletion::*;
pub use imports::{
    ImportCanvasVideoInput, UploadAssemblyInsertVideoInput, UploadChapterVideoInput,
    UploadReferenceMediaVideoInput, assemble_canvas_videos, extract_canvas_video_frame,
    import_canvas_video, trim_video, upload_assembly_insert_video, upload_chapter_video,
    upload_reference_media_video,
};
use playback::MAX_INLINE_PLAYBACK_VIDEO_BYTES;
pub use playback::{load_video_playback_data, load_video_scrub_playback_data};
pub use records::*;

#[cfg(test)]
mod tests;
