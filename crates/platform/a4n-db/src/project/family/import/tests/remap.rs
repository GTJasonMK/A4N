use super::super::remap::{
    remap_manga_generated_image_metadata, remap_media_binding_target_id,
    remap_media_source_external_path, remap_source_sync_target_ref, remap_video_metadata,
};
use super::super::*;
use a4n_db_models::models::{
    generation::generation_spec,
    media::media_binding,
    project::{source_snapshot_outline, source_snapshot_profile},
    source_sync::source_sync_conflict,
};
use a4n_domain::source_sync::SourceSyncConflictStatus;
use chrono::NaiveDate;
use serde_json::json;
use std::collections::HashMap;

mod fixtures {
    mod media_generation;
    mod source_snapshot;
    mod source_sync;

    pub(super) use media_generation::*;
    pub(super) use source_snapshot::*;
    pub(super) use source_sync::*;
}

use fixtures::*;

fn dt() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap()
}

mod generation;
mod media;
mod snapshot;
mod source_sync;
