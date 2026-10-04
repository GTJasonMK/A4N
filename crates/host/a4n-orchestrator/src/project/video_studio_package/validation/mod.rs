//! 视频画布项目包纯校验子树。
//!
//! 每个校验子模块显式声明依赖，避免通过父模块 glob 导入隐藏合同。

use crate::project::{ImportMediaPayload, VideoStudioProjectPackage};

mod canvas;
mod manifest;
pub(crate) use manifest::validate_canvas_media_asset_owner;
mod media;
mod provenance;

#[cfg(test)]
pub(crate) use media::{MediaEntryOwnerIndex, validate_media_entry_bytes};

pub(super) fn validate_package_manifest(
    package: &VideoStudioProjectPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    manifest::validate_package_manifest(package, media_payload)
}

#[cfg(test)]
mod tests;
