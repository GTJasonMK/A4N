//! 跨 host 模块复用的本地媒体合同与纯探测工具。
//!
//! 本 crate 不拥有数据库、provider、权限、项目路径或媒体文件生命周期；
//! 调用方仍负责决定允许探测的路径，并把 typed error 映射为业务错误。

mod media_file;
mod upload;

pub use a4n_media_processing_core::{
    AudioProbeError, FfmpegCommandError, FfmpegFrameError, FfmpegTrimError, FfprobeCommandError,
    VideoAssemblyClip, VideoAssemblyError, VideoAssemblyRequest, VideoPostprocessError,
    VideoPostprocessRequest, VideoProbeError, VideoProbeProperties, assemble_videos,
    build_video_assembly_filter, build_video_postprocess_filter, extract_video_frame,
    extract_video_tail_frame, generate_video_thumbnail, parse_audio_duration_secs,
    parse_frame_rate, parse_video_probe_properties, postprocess_video, probe_audio_duration_secs,
    probe_video_has_audio, probe_video_properties, trim_video_segment,
};
pub use a4n_media_render_core::{
    VIDEO_SCRUB_FRAMES_PER_SECOND, VideoScrubProxyError, generate_video_scrub_frames,
    generate_video_scrub_proxy,
};
pub use media_file::{
    MediaFileResponse, MediaFileResponseError, MediaResponseStatus, read_media_file_response,
    relative_media_path, relative_media_uri_path, requested_byte_range,
    requested_byte_range_header,
};
pub use upload::{
    ImageDimensionsError, MAX_IMPORT_AUDIO_BYTES, decode_uploaded_audio_data,
    decode_uploaded_bytes, decode_uploaded_image_data, image_dimensions_from_bytes,
    required_upload_file_name,
};
