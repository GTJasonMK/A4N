use super::super::super::ProjectFamilyPackage;
use super::super::{
    anchors::{validate_profile_evidence_manifest, validate_text_anchor_manifest},
    galgame::validate_galgame_manifest,
    generation::validate_generation_manifest,
    source_snapshots::validate_source_snapshot_manifest,
    source_sync::validate_source_sync_manifest,
    text_core::{
        validate_text_core_chapter_outline_part_references,
        validate_text_core_chapter_outline_references, validate_text_core_chapter_outline_statuses,
        validate_text_core_part_outline_ranges,
    },
};
use super::{
    derivations::validate_package_derivations, projects::validate_package_projects,
    uniqueness::validate_package_uniqueness,
};

pub(in super::super::super) fn validate_package_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    validate_package_projects(package)?;
    validate_package_uniqueness(package)?;
    validate_source_snapshot_manifest(package)?;
    validate_package_derivations(package)?;
    validate_text_core_part_outline_ranges(package)?;
    validate_text_core_chapter_outline_statuses(package)?;
    validate_text_core_chapter_outline_part_references(package)?;
    validate_text_core_chapter_outline_references(package)?;
    validate_text_anchor_manifest(package)?;
    validate_profile_evidence_manifest(package)?;
    validate_galgame_manifest(package)?;
    validate_generation_manifest(package)?;
    validate_source_sync_manifest(package)
}
