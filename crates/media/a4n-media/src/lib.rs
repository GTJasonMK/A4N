//! 跨 host 模块复用的媒体文件读取、上传校验与数据投影。
//!
//! 本 crate 不拥有数据库、provider、权限、项目路径或媒体文件生命周期；
//! 调用方仍负责决定允许探测的路径，并把 typed error 映射为业务错误。

mod media_file;
mod upload;

pub use a4n_media_contract::{VIDEO_SCRUB_FRAMES_PER_SECOND, VideoProbeProperties};
pub use media_file::{
    MediaFileResponse, MediaFileResponseError, MediaResponseStatus, read_media_file_response,
    relative_media_path, relative_media_uri_path, requested_byte_range,
    requested_byte_range_header,
};
pub use upload::{
    ImageDimensionsError, MAX_IMPORT_AUDIO_BYTES, decode_uploaded_audio_data,
    decode_uploaded_bytes, decode_uploaded_bytes_to_writer, decode_uploaded_image_data,
    image_dimensions_from_bytes, image_upload_format_from_image_format, required_upload_file_name,
};

pub fn video_file_properties(
    probe: &VideoProbeProperties,
    file_size: i64,
) -> a4n_domain::media::VideoFileProperties {
    a4n_domain::media::VideoFileProperties {
        width: probe.width,
        height: probe.height,
        duration_secs: probe.duration_secs,
        fps: probe.fps,
        file_size,
    }
}
