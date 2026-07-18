use crate::state::AppState;
#[cfg(test)]
use a4n_core::models::{
    asset::object_profile,
    chapter::{chapter, part_outline},
    character::protagonist_profile,
    generation::{
        generation_spec_entity_ref, generation_spec_media_ref, generation_spec_text_anchor,
    },
    media::media_provenance,
    project::{
        profile_evidence, project_derivation, source_snapshot_outline, source_snapshot_profile,
    },
    scene::scene_profile,
    video_project::{assembly_timeline, video_segment},
};
use a4n_core::models::{
    chapter::chapter_outline,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding},
    project::{novel_project, project_source_snapshot, source_snapshot_chapter, text_anchor},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
};
#[cfg(test)]
use a4n_orchestrator::media::MEDIA_STATUS_PENDING;
use a4n_orchestrator::media::{
    MEDIA_STATUS_COMPLETED, galgame_media_binding_required_media_kind,
    is_valid_galgame_media_binding_pair,
};
use a4n_orchestrator::source_sync::CONFLICT_STATUS_PENDING;
#[cfg(test)]
use a4n_orchestrator::source_sync::{
    CONFLICT_STATUS_RESOLVED, SYNC_RUN_MODE_APPLY, SYNC_RUN_MODE_DRY_RUN,
    SYNC_RUN_STATUS_COMPLETED, SYNC_RUN_STATUS_PLANNED,
};
#[cfg(test)]
use a4n_orchestrator::text_anchor::{EVIDENCE_ROLE_FIRST_SEEN, SOURCE_KIND_CHAPTER_CONTENT};
#[cfg(test)]
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

mod archive;
mod contracts;
mod export;
mod import_rows;
#[cfg(test)]
mod remap;
mod rules;
mod validation;

use contracts::*;
use rules::*;

pub(super) async fn export_project_family_package_bytes(
    state: &AppState,
    root_project_id: i64,
    include_derived_projects: bool,
) -> Result<Vec<u8>, String> {
    export::export_project_family_package_bytes(state, root_project_id, include_derived_projects)
        .await
}

pub(super) async fn export_derived_project_package_bytes(
    state: &AppState,
    derived_project_id: i64,
) -> Result<Vec<u8>, String> {
    export::export_derived_project_package_bytes(state, derived_project_id).await
}

pub(super) async fn import_project_family_package_bytes(
    state: &AppState,
    user_id: i64,
    bytes: Vec<u8>,
) -> Result<Vec<novel_project::Model>, String> {
    archive::import_project_family_package_bytes(state, user_id, bytes).await
}

#[cfg(test)]
mod tests;
