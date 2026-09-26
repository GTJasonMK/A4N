use a4n_db_models::models::chapter::{chapter, chapter_visual_plan};
use serde_json::Value;
use std::collections::HashMap;

pub(crate) use a4n_domain::project::SourceSnapshotChapterSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedChapterScope {
    pub start_chapter: i32,
    pub end_chapter: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateDerivedProjectParams {
    pub root_project_id: i64,
    pub derived_kind: String,
    pub title: String,
    pub description: String,
    pub chapter_scope: DerivedChapterScope,
}

pub(super) struct SourceSnapshotBuildData {
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) visual_plans: HashMap<i64, chapter_visual_plan::Model>,
    pub(super) shooting_scripts: HashMap<i64, SourceSnapshotShootingScript>,
    pub(super) profiles: Vec<SourceSnapshotProfile>,
    pub(super) chapter_hashes_json: Vec<Value>,
    pub(super) profile_hashes_json: Value,
    pub(super) latest_blueprint_version_id: Option<i64>,
    pub(super) chapter_scope_json: Value,
    pub(super) source_version_hash: String,
}

pub(super) struct SourceSnapshotShootingScript {
    pub(super) script_json: Value,
    pub(super) input_fingerprint_sha256: String,
}

pub(super) struct SourceSnapshotProfile {
    pub(super) kind: &'static str,
    pub(super) id: i64,
    pub(super) payload: Value,
}
