//! 视频画布项目包 ZIP、manifest 与媒体 payload 适配。

pub(super) type ImportMediaPayload = a4n_orchestrator::project::ImportMediaPayload;
#[cfg(test)]
pub(super) type VideoStudioMediaFileEntry = a4n_orchestrator::project::VideoStudioMediaFileEntry;
pub(super) type VideoStudioProjectPackage = a4n_orchestrator::project::VideoStudioProjectPackage;

pub(super) const MANIFEST_PATH: &str = a4n_orchestrator::project::MANIFEST_PATH;
#[cfg(test)]
pub(super) const MEDIA_FIELD_FILE: &str = a4n_orchestrator::project::MEDIA_FIELD_FILE;
#[cfg(test)]
pub(super) const MEDIA_OWNER_MEDIA_ASSET: &str = a4n_orchestrator::project::MEDIA_OWNER_MEDIA_ASSET;
#[cfg(test)]
pub(super) const PACKAGE_MODE: &str = a4n_orchestrator::project::PACKAGE_MODE;
#[cfg(test)]
pub(super) const VIDEO_STUDIO_PACKAGE_VERSION: &str =
    a4n_orchestrator::project::VIDEO_STUDIO_PACKAGE_VERSION;

mod package_io;

pub(super) use package_io::{package_zip_bytes, read_video_studio_package_bytes};
