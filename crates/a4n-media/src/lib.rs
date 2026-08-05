//! 跨 host 模块复用的本地媒体合同与纯探测工具。
//!
//! 本 crate 不拥有数据库、provider、权限、项目路径或媒体文件生命周期；
//! 调用方仍负责决定允许探测的路径，并把 typed error 映射为业务错误。

mod assembly;
mod ffmpeg;
mod playback;
mod postprocess;
mod probe;

pub use assembly::{
    VideoAssemblyClip, VideoAssemblyError, VideoAssemblyRequest, assemble_videos,
    build_video_assembly_filter, ffmpeg_duration_literal,
};
pub use ffmpeg::{
    FfmpegAvailabilityError, FfmpegCommandError, FfmpegTrimError, check_ffmpeg_available,
    extract_video_frame, extract_video_tail_frame, generate_video_thumbnail, trim_video_segment,
};
pub use playback::{VideoScrubProxyError, generate_video_scrub_proxy};
pub use postprocess::{
    VideoPostprocessError, VideoPostprocessOperation, VideoPostprocessRequest,
    build_video_postprocess_filter, postprocess_video,
};
pub use probe::{
    AudioProbeError, FfprobeCommandError, VideoProbeError, VideoProbeProperties,
    parse_audio_duration_secs, parse_frame_rate, parse_video_probe_properties,
    probe_audio_duration_secs, probe_video_has_audio, probe_video_properties,
};

#[cfg(test)]
mod tests;
