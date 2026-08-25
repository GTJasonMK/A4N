//! 视频画布项目包 ZIP、manifest 与媒体 payload 适配。

mod package_io;

pub(super) use package_io::{package_zip_bytes, read_video_studio_package_bytes};
