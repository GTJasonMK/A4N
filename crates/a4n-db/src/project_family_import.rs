mod contracts;
mod errors;
mod execution;
mod import_rows;
mod remap;
mod rules;

pub use contracts::*;
pub use errors::*;
pub use execution::import_project_family_package_rows;
pub use remap::*;
pub use rules::{
    map_optional, map_required, media_asset_status_for_import_insert, remap_optional_entity_id,
    remap_profile_id,
};

pub(in crate::project_family_import) use rules::{hash_json, normalize_profile_evidence_role};

use a4n_core::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    media::media_binding,
    project::novel_project,
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, NotSet, Set};
use serde_json::Value;

#[cfg(test)]
use sea_orm::DbErr;

#[cfg(test)]
mod tests;
