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
    validate_package_manifest_inner(package, &std::collections::HashSet::new())
}

pub(crate) fn validate_package_manifest_with_canvas(
    package: &ProjectFamilyPackage,
    canvases: &[crate::project::VideoStudioProjectPackage],
) -> Result<(), String> {
    let mut validated_canvas_specs = std::collections::HashSet::new();
    for canvas in canvases.iter().filter(|canvas| {
        package
            .projects
            .iter()
            .any(|project| project.id == canvas.project.id)
    }) {
        validated_canvas_specs.extend(
            crate::project::video_studio_package::validate_family_canvas_package(canvas, package)?,
        );
    }
    validate_package_manifest_inner(package, &validated_canvas_specs)
}

fn validate_package_manifest_inner(
    package: &ProjectFamilyPackage,
    validated_canvas_specs: &std::collections::HashSet<i64>,
) -> Result<(), String> {
    validate_package_projects(package)?;
    validate_package_uniqueness(package)?;
    validate_source_snapshot_manifest(package)?;
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
    validate_generation_manifest(package, validated_canvas_specs)?;
    validate_source_sync_manifest(package)
}
