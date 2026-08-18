//! 视频画布项目包 ZIP、manifest 与媒体 payload 适配。

pub(super) type ImportMediaPayload = a4n_orchestrator::project::ImportMediaPayload;
pub(super) type VideoStudioProjectPackage = a4n_orchestrator::project::VideoStudioProjectPackage;

pub(super) const MANIFEST_PATH: &str = a4n_orchestrator::project::MANIFEST_PATH;

mod package_io;

pub(super) use package_io::{package_zip_bytes, read_video_studio_package_bytes};
