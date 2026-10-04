use super::super::super::ProjectFamilyPackage;
use super::super::{
    anchors::{validate_profile_evidence_manifest, validate_text_anchor_manifest},
    galgame::validate_galgame_manifest,
    generation::validate_generation_manifest,
    manga::validate_manga_manifest,
    source_snapshots::validate_source_snapshot_manifest,
    source_sync::validate_source_sync_manifest,
    text_core::{
        validate_text_core_chapter_outline_part_references,
        validate_text_core_chapter_outline_references, validate_text_core_chapter_outline_statuses,
        validate_text_core_part_outline_ranges,
    },
    video::validate_video_manifest,
    visual_plan::validate_visual_plan_manifest,
};
use super::{projects::validate_package_projects, uniqueness::validate_package_uniqueness};

pub fn validate_package_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    validate_package_manifest_inner(package)
}

fn validate_package_manifest_inner(package: &ProjectFamilyPackage) -> Result<(), String> {
    validate_package_projects(package)?;
    validate_package_uniqueness(package)?;
    validate_source_snapshot_manifest(package)?;
    super::super::authoring::validate_authoring_manifest(package)?;
    validate_visual_plan_manifest(package)?;
    validate_text_core_part_outline_ranges(package)?;
    validate_text_core_chapter_outline_statuses(package)?;
    validate_text_core_chapter_outline_part_references(package)?;
    validate_text_core_chapter_outline_references(package)?;
    validate_text_anchor_manifest(package)?;
    validate_profile_evidence_manifest(package)?;
    validate_manga_manifest(package)?;
    validate_video_manifest(package)?;
    validate_galgame_manifest(package)?;
    validate_generation_manifest(package, &std::collections::HashSet::new())?;
    validate_source_sync_manifest(package)
}
