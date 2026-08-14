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
mod common;
mod galgame;
mod generation;
mod media;
mod package;
mod source_snapshots;
mod source_sync;
mod text_core;

pub use common::{snapshot_profile_key, validate_media_bytes};
pub use media::validate_media_manifest;
pub use package::validate_package_manifest;
