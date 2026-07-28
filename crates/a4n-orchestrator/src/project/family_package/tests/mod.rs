//! 项目族包纯校验单测（无 Host/DB）。

use super::validation::{
    validate_media_bytes, validate_media_manifest, validate_package_entry_path,
    validate_package_manifest,
};
use super::*;
use crate::generation::{
    GENERATION_ENTITY_ROLES, GENERATION_MEDIA_REF_ROLES, GENERATION_PROMPT_KINDS,
    GENERATION_SPEC_STATUSES, GENERATION_SPEC_TARGET_ENTITY_KINDS, GENERATION_SPEC_TARGET_KINDS,
    GENERATION_TEXT_ANCHOR_ROLES,
};
use crate::media::{
    MEDIA_BINDING_ROLES, MEDIA_BINDING_TARGET_KINDS, MEDIA_KIND_AUDIO, MEDIA_KIND_IMAGE,
    MEDIA_KIND_VIDEO, MEDIA_ORIGIN_ASSEMBLED_PREVIEW, MEDIA_ORIGIN_EXTRACTED_FRAME,
    MEDIA_ORIGIN_GENERATED, MEDIA_ORIGIN_IMPORTED, MEDIA_STATUS_CANCELLED, MEDIA_STATUS_FAILED,
    MEDIA_STATUS_RUNNING,
};
use crate::project::{
    DERIVED_PROJECT_STATUS_DRAFT, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO, SOURCE_SYNC_CONFLICT,
    SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED,
    media_asset_status_for_import_insert,
};
use crate::source_sync::{
    CONFLICT_SEVERITIES, CONFLICT_STATUSES, DECISION_ACTORS, LOCAL_STATE_KINDS,
    RESOLUTION_STRATEGIES, SOURCE_CHANGE_KINDS, SYNC_RUN_STATUSES,
};
use chrono::NaiveDate;

mod fixtures {
    mod galgame;
    mod media_generation;
    mod project;
    mod source_snapshot;
    mod source_sync;
    mod story;

    pub(super) use galgame::*;
    pub(super) use media_generation::*;
    pub(super) use project::*;
    pub(super) use source_snapshot::*;
    pub(super) use source_sync::*;
    pub(super) use story::*;
}

use fixtures::*;

fn dt() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap()
}

const PROJECT_KIND_GALGAME_TEST: &str = "galgame";

mod core_validation;
mod media_validation;
mod source_sync_validation;
