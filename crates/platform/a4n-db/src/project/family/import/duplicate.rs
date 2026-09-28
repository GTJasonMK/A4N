//! Copy editable project artifacts through the same graph mapper as archive
//! import. Immutable root-owned sources and materials keep their identity.

use std::collections::HashMap;

use a4n_db_models::models::{chapter::chapter_shooting_script, project::novel_project};
use a4n_domain::{media_authoring::AuthoringDocumentInputs, project::ProjectMediaFileCopier};
use sea_orm::{DatabaseTransaction, EntityTrait};

use super::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage,
    ProjectFamilyMediaFileImporter, ProjectFamilyMediaImportError, import_rows,
};

pub(crate) async fn copy_derived_media_rows<Copier: ProjectMediaFileCopier + ?Sized>(
    txn: &DatabaseTransaction,
    original: &novel_project::Model,
    copy: &novel_project::Model,
    copier: &mut Copier,
) -> Result<(), ProjectFamilyImportError> {
    let root_id = original
        .root_project_id
        .ok_or_else(|| invalid("复制派生项目缺少原作"))?;
    let snapshot_id = original
        .source_snapshot_id
        .ok_or_else(|| invalid("复制派生项目缺少冻结原文"))?;
    let rows = crate::project::family::export::project_family_export_rows(
        txn,
        root_id,
        &[original.id],
        &[snapshot_id],
    )
    .await
    .map_err(|error| match error {
        crate::project::ProjectFamilyExportError::Database(error) => {
            ProjectFamilyImportError::Database(error)
        }
        error => invalid(error),
    })?;
    if !rows
        .source_snapshots
        .iter()
        .any(|row| row.id == snapshot_id && row.root_project_id == root_id)
    {
        return Err(invalid("复制派生项目的冻结原文不属于该原作"));
    }
    let mut package = ProjectFamilyImportPackage {
        canvas_projects: Vec::new(),
        root_project_id: root_id,
        projects: vec![original.clone()],
        text_core: rows.text_core,
        source_snapshots: rows.source_snapshots,
        source_snapshot_chapters: rows.source_snapshot_chapters,
        source_snapshot_outlines: rows.source_snapshot_outlines,
        source_snapshot_profiles: rows.source_snapshot_profiles,
        text_anchors: rows.text_anchors,
        profile_evidence: rows.profile_evidence,
        chapter_visual_plans: rows.chapter_visual_plans,
        chapter_shooting_scripts: rows.chapter_shooting_scripts,
        authoring_documents: rows.authoring_documents,
        authoring_receipts: rows.authoring_receipts,
        character_portraits: rows.character_portraits,
        generated_images: rows.generated_images,
        chapter_manga_prompts: rows.chapter_manga_prompts,
        chapter_video_prompts: rows.chapter_video_prompts,
        generated_videos: rows.generated_videos,
        generation_specs: rows.generation_specs,
        generation_spec_text_anchors: rows.generation_spec_text_anchors,
        generation_spec_entity_refs: rows.generation_spec_entity_refs,
        generation_spec_media_refs: rows.generation_spec_media_refs,
        prompt_artifacts: rows.prompt_artifacts,
        media_assets: rows.media_assets,
        media_provenance: rows.media_provenance,
        media_bindings: rows.media_bindings,
        galgame_scripts: rows.galgame_scripts,
        galgame_scenes: rows.galgame_scenes,
        galgame_lines: rows.galgame_lines,
        galgame_choices: rows.galgame_choices,
        video_segments: rows.video_segments,
        assembly_timelines: rows.assembly_timelines,
        assembly_clips: rows.assembly_clips,
        source_sync_runs: rows.source_sync_runs,
        source_sync_conflicts: rows.source_sync_conflicts,
        source_sync_decisions: rows.source_sync_decisions,
    };
    let mut maps = ImportIdMaps::default();
    maps.project_ids
        .extend([(root_id, root_id), (original.id, copy.id)]);
    maps.chapter_ids.extend(
        package
            .text_core
            .chapters
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.blueprint_version_ids.extend(
        package
            .text_core
            .blueprint_versions
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.part_outline_ids.extend(
        package
            .text_core
            .part_outlines
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.chapter_outline_ids.extend(
        package
            .text_core
            .chapter_outlines
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.source_part_outline_ids = maps.part_outline_ids.clone();
    maps.source_chapter_outline_ids = maps.chapter_outline_ids.clone();
    maps.character_profile_ids.extend(
        package
            .text_core
            .character_profiles
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.object_profile_ids.extend(
        package
            .text_core
            .object_profiles
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.scene_profile_ids.extend(
        package
            .text_core
            .scene_profiles
            .iter()
            .filter(|row| row.project_id == root_id)
            .map(|row| (row.id, row.id)),
    );
    maps.scene_occurrence_ids.extend(
        package
            .text_core
            .scene_occurrences
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.source_snapshot_ids
        .extend(package.source_snapshots.iter().map(|row| (row.id, row.id)));
    maps.source_snapshot_version_hashes.extend(
        package
            .source_snapshots
            .iter()
            .map(|row| (row.id, row.source_version_hash.clone())),
    );
    maps.source_snapshot_chapter_ids.extend(
        package
            .source_snapshot_chapters
            .iter()
            .map(|row| (row.id, row.id)),
    );
    maps.text_anchor_ids
        .extend(package.text_anchors.iter().map(|row| (row.id, row.id)));
    for image in package
        .generated_images
        .iter()
        .filter(|image| image.project_id != original.id)
    {
        maps.generated_image_ids.insert(image.id, image.id);
        maps.generated_image_paths
            .insert(image.id, image.file_path.clone());
    }
    for portrait in package
        .character_portraits
        .iter()
        .filter(|row| row.project_id != original.id)
    {
        maps.character_portrait_ids.insert(portrait.id, portrait.id);
        maps.character_portrait_paths
            .insert(portrait.id, portrait.image_path.clone());
    }
    // A local copy can still reach the original video. Seed its unchanged
    // identity so a frozen reference is not detached as it would be in an export
    // that omitted its producer.
    for row in &package.authoring_documents {
        let inputs: AuthoringDocumentInputs =
            serde_json::from_value(row.inputs_json.clone()).map_err(invalid)?;
        let Some(reference) = inputs.video_reference else {
            continue;
        };
        let Some(video_id) = reference.video_project_id else {
            continue;
        };
        if let Some(script) = chapter_shooting_script::Entity::find_by_id(reference.script_id)
            .one(txn)
            .await?
            && script.project_id == video_id
            && script.root_project_id == root_id
        {
            maps.project_ids.insert(video_id, video_id);
            maps.shooting_script_ids.insert(script.id, script.id);
            maps.shooting_script_sha256s
                .insert(script.id, script.script_sha256.clone());
            maps.shooting_script_sources
                .insert(script.id, (&script).into());
        }
    }
    package
        .generated_images
        .retain(|row| row.project_id == original.id);
    package
        .character_portraits
        .retain(|row| row.project_id == original.id);
    package
        .chapter_manga_prompts
        .retain(|row| row.project_id == original.id);
    package
        .chapter_video_prompts
        .retain(|row| row.project_id == original.id);
    package
        .generated_videos
        .retain(|row| row.project_id == original.id);
    package
        .authoring_receipts
        .retain(|row| row.project_id == original.id);
    import_rows::insert_scene_profiles(txn, &package, &mut maps).await?;
    let mut writer = CopyMediaWriter {
        copier,
        paths: media_paths(&package),
    };
    import_rows::insert_media_project_rows(copy.user_id, txn, &package, &mut maps, &mut writer)
        .await
}

fn media_paths(package: &ProjectFamilyImportPackage) -> HashMap<(String, i64, String), String> {
    let mut paths = HashMap::new();
    let mut add = |kind: &str, id: i64, field: &str, path: Option<&str>| {
        if let Some(path) = path {
            paths.insert((kind.into(), id, field.into()), path.into());
        }
    };
    for row in &package.generated_images {
        add("generated_image", row.id, "file_path", Some(&row.file_path));
    }
    for row in &package.character_portraits {
        add(
            "character_portrait",
            row.id,
            "image_path",
            Some(&row.image_path),
        );
    }
    for row in &package.generated_videos {
        add(
            "generated_video",
            row.id,
            "file_path",
            row.file_path.as_deref(),
        );
        add(
            "generated_video",
            row.id,
            "thumbnail_path",
            row.thumbnail_path.as_deref(),
        );
    }
    for row in &package.media_assets {
        add("media_asset", row.id, "file_path", row.file_path.as_deref());
        add(
            "media_asset",
            row.id,
            "thumbnail_path",
            row.thumbnail_path.as_deref(),
        );
    }
    paths
}

struct CopyMediaWriter<'a, Copier: ?Sized> {
    copier: &'a mut Copier,
    paths: HashMap<(String, i64, String), String>,
}

impl<Copier: ProjectMediaFileCopier + ?Sized> ProjectFamilyMediaFileImporter
    for CopyMediaWriter<'_, Copier>
{
    fn write_imported_media_field(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        old_owner_id: i64,
        new_owner_id: i64,
        field: &str,
    ) -> Result<Option<String>, ProjectFamilyMediaImportError> {
        let path = self
            .paths
            .get(&(owner_kind.into(), old_owner_id, field.into()));
        self.copier
            .copy_media_file(project_id, new_owner_id, field, path.map(String::as_str))
            .map_err(|error| ProjectFamilyMediaImportError::Write(error.to_string()))
    }
}

fn invalid(error: impl std::fmt::Display) -> ProjectFamilyImportError {
    ProjectFamilyImportError::InvalidPackage(error.to_string())
}
