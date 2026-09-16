//! FFmpeg/FFprobe implementation owned by the replaceable Media Worker.

mod assembly;
mod ffmpeg;
mod postprocess;
mod probe;
mod worker;

pub use a4n_media_render_core::{
    VIDEO_SCRUB_FRAMES_PER_SECOND, VideoScrubProxyError, generate_video_scrub_frames,
    generate_video_scrub_proxy,
};
pub use assembly::{
    VideoAssemblyClip, VideoAssemblyError, VideoAssemblyRequest, assemble_videos,
    build_video_assembly_filter,
};
pub use ffmpeg::{
    FfmpegCommandError, FfmpegFrameError, FfmpegTrimError, extract_video_frame,
    extract_video_tail_frame, generate_video_thumbnail, trim_video_segment,
};
pub use postprocess::{
    VideoPostprocessError, VideoPostprocessRequest, build_video_postprocess_filter,
    postprocess_video,
};
pub use probe::{
    AudioProbeError, FfprobeCommandError, VideoProbeError, VideoProbeProperties,
    parse_audio_duration_secs, parse_frame_rate, parse_video_probe_properties,
    probe_audio_duration_secs, probe_video_has_audio, probe_video_properties,
};
pub use worker::execute;

#[cfg(test)]
mod tests;
