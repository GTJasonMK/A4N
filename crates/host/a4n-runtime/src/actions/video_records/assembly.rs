mod actions;
mod cache;
mod contracts;
mod postprocess;
mod push;

pub(crate) use cache::recover_expired_assembly_caches;

pub use a4n_api_contract::video::UpdateAssemblyPushStatusInput;
#[cfg(test)]
pub(in crate::actions::video_records) use actions::persist_assembly_preview_cache;
pub use actions::{
    apply_assembly_postprocess_edit, assemble_chapter_videos,
    load_cached_assembled_chapter_video_preview, push_assembled_chapter_video,
    update_assembly_push_status,
};
pub use contracts::{
    ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
    AssembledChapterVideoPreview, AssemblyPushResult, PushAssembledChapterVideoInput,
};

#[cfg(test)]
pub(in crate::actions::video_records) use cache::{
    assembly_cache_is_fresh, build_assembly_cache_manifest_with_audio_probe,
    expired_assembly_cache_files,
};
#[cfg(test)]
pub(in crate::actions::video_records) use cache::{
    assembly_cache_matches, write_assembly_cache_manifest,
};
pub(in crate::actions::video_records) use cache::{
    build_assembly_cache_manifest, run_ffmpeg_assemble_videos,
};
#[cfg(test)]
pub(in crate::actions::video_records) use contracts::AssemblyCacheManifest;
pub(in crate::actions::video_records) use contracts::AssemblyClip;
