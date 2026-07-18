use super::*;

const GALGAME_CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];

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

pub(super) use common::{snapshot_profile_key, validate_media_bytes, validate_package_entry_path};
pub(super) use media::validate_media_manifest;
pub(super) use package::validate_package_manifest;
