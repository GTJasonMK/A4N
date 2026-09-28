//! 项目族包纯校验子树。

use super::*;

macro_rules! ensure_package {
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            return Err(format!($($arg)*));
        }
    };
}

mod anchors;
mod authoring;
mod common;
mod galgame;
mod generation;
mod manga;
mod media;
mod package;
mod source_snapshots;
mod source_sync;
mod text_core;
mod video;
mod visual_plan;

#[cfg(test)]
pub(crate) use common::validate_media_bytes;
pub(crate) use media::validate_media_manifest;
pub(crate) use package::{validate_package_manifest, validate_package_manifest_with_canvas};
