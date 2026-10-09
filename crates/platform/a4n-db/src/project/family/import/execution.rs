use a4n_domain::project::ProjectRecord;
use sea_orm::{ConnectionTrait, DbBackend, Statement};

use crate::project::project_record_from_model;

use super::{
    DatabaseProjectFamilyImportPackage, ProjectFamilyImportError, ProjectFamilyImportMappings,
    ProjectFamilyImportPackage, ProjectFamilyMediaFileImporter, import_rows,
};
use serde::{Serialize, de::DeserializeOwned};

pub async fn import_project_family_package_rows<Importer>(
    db: &impl crate::DbExecutor,
    user_id: i64,
    package: &ProjectFamilyImportPackage,
    media_importer: &mut Importer,
) -> Result<Vec<ProjectRecord>, ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    let (imported_projects, _) =
        import_project_family_package_rows_with_mappings(db, user_id, package, media_importer)
            .await?;
    Ok(imported_projects)
}

pub async fn import_project_family_package_rows_with_mappings<Importer>(
    db: &impl crate::DbExecutor,
    user_id: i64,
    package: &ProjectFamilyImportPackage,
    media_importer: &mut Importer,
) -> Result<(Vec<ProjectRecord>, ProjectFamilyImportMappings), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    let database_package = database_package(package)?;
    let txn = db.begin().await?;
    let (imported_projects, maps) =
        import_rows::import_package_rows(user_id, &txn, &database_package, media_importer).await?;
    for project in imported_projects
        .iter()
        .filter(|project| project.project_kind == "galgame")
    {
        let invalid_scene = txn.query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT scene.id FROM galgame_scene scene
            JOIN galgame_script game ON game.id = scene.script_id
            JOIN source_snapshot_chapter source ON source.id = scene.source_snapshot_chapter_id
            WHERE game.project_id = $1 AND (
              (scene.entry_kind = 'main' AND scene.adaptation_plan_id IS NULL)
              OR ((scene.adaptation_plan_id IS NULL) <> (scene.adaptation_unit_id IS NULL))
              OR (scene.adaptation_plan_id IS NOT NULL AND NOT EXISTS (
                SELECT 1 FROM media_authoring_document plan
                CROSS JOIN LATERAL jsonb_array_elements(plan.document_json::jsonb -> 'units') item(value)
                WHERE plan.id = scene.adaptation_plan_id
                  AND plan.project_id = game.project_id
                  AND plan.source_snapshot_id = source.snapshot_id
                  AND plan.source_chapter_id = source.source_chapter_id
                  AND plan.kind = 'adaptation_plan'
                  AND plan.status IN ('adopted', 'superseded')
                  AND item.value ->> 'id' = scene.adaptation_unit_id
              ))) LIMIT 1"#,
            [project.id.into()],
        )).await?;
        if let Some(scene) = invalid_scene {
            let scene_id: i64 = scene.try_get("", "id")?;
            return Err(ProjectFamilyImportError::InvalidPackage(format!(
                "Galgame 场 {scene_id} 的改编单元不属于导入的本地改编计划"
            )));
        }
    }
    txn.commit()
        .await
        .map_err(ProjectFamilyImportError::Commit)?;
    Ok((
        imported_projects
            .into_iter()
            .map(project_record_from_model)
            .collect(),
        ProjectFamilyImportMappings {
            entity_ids: [
                ("chapter_visual_plan", maps.visual_plan_ids),
                ("chapter_shooting_script", maps.shooting_script_ids),
                ("media_authoring_document", maps.authoring_document_ids),
                ("chapter_manga_prompt", maps.chapter_manga_prompt_ids),
                ("chapter_video_prompt", maps.chapter_video_prompt_ids),
                ("galgame_script", maps.galgame_script_ids),
                ("galgame_scene", maps.galgame_scene_ids),
                ("galgame_line", maps.galgame_line_ids),
                ("generation_spec", maps.generation_spec_ids),
                ("prompt_artifact", maps.prompt_artifact_ids),
                ("media_asset", maps.media_asset_ids),
                ("generated_image", maps.generated_image_ids),
                ("generated_video", maps.generated_video_ids),
                ("character_portrait", maps.character_portrait_ids),
                ("protagonist_profile", maps.character_profile_ids),
                ("object_profile", maps.object_profile_ids),
                ("scene_profile", maps.scene_profile_ids),
                ("text_anchor", maps.text_anchor_ids),
                ("video_segment", maps.video_segment_ids),
                ("assembly_timeline", maps.assembly_timeline_ids),
                ("assembly_clip", maps.assembly_clip_ids),
                ("blueprint_version", maps.blueprint_version_ids),
                ("novel_blueprint", maps.blueprint_ids),
                ("part_outline", maps.part_outline_ids),
                ("chapter_outline", maps.chapter_outline_ids),
            ]
            .into_iter()
            .map(|(table, ids)| (table.into(), ids))
            .collect(),
            chapter_ids: maps.chapter_ids,
            source_snapshot_ids: maps.source_snapshot_ids,
            source_snapshot_chapter_ids: maps.source_snapshot_chapter_ids,
        },
    ))
}

pub(super) fn database_package(
    package: &ProjectFamilyImportPackage,
) -> Result<DatabaseProjectFamilyImportPackage, ProjectFamilyImportError> {
    Ok(DatabaseProjectFamilyImportPackage {
        root_project_id: package.root_project_id,
        projects: package.projects.clone(),
        text_core: database_record(&package.text_core)?,
        source_snapshots: database_record(&package.source_snapshots)?,
        source_snapshot_chapters: database_record(&package.source_snapshot_chapters)?,
        source_snapshot_outlines: database_record(&package.source_snapshot_outlines)?,
        source_snapshot_profiles: database_record(&package.source_snapshot_profiles)?,
        text_anchors: database_record(&package.text_anchors)?,
        profile_evidence: database_record(&package.profile_evidence)?,
        chapter_visual_plans: database_record(&package.chapter_visual_plans)?,
        chapter_shooting_scripts: database_record(&package.chapter_shooting_scripts)?,
        authoring_documents: database_record(&package.authoring_documents)?,
        authoring_receipts: database_record(&package.authoring_receipts)?,
        character_portraits: database_record(&package.character_portraits)?,
        generated_images: database_record(&package.generated_images)?,
        chapter_manga_prompts: database_record(&package.chapter_manga_prompts)?,
        manga_pipeline_artifacts: database_record(&package.manga_pipeline_artifacts)?,
        chapter_video_prompts: database_record(&package.chapter_video_prompts)?,
        generated_videos: database_record(&package.generated_videos)?,
        generation_specs: database_record(&package.generation_specs)?,
        generation_spec_text_anchors: database_record(&package.generation_spec_text_anchors)?,
        generation_spec_entity_refs: database_record(&package.generation_spec_entity_refs)?,
        generation_spec_media_refs: database_record(&package.generation_spec_media_refs)?,
        prompt_artifacts: database_record(&package.prompt_artifacts)?,
        media_assets: database_record(&package.media_assets)?,
        media_provenance: database_record(&package.media_provenance)?,
        media_bindings: database_record(&package.media_bindings)?,
        galgame_scripts: database_record(&package.galgame_scripts)?,
        galgame_scenes: database_record(&package.galgame_scenes)?,
        galgame_lines: database_record(&package.galgame_lines)?,
        galgame_choices: database_record(&package.galgame_choices)?,
        video_segments: database_record(&package.video_segments)?,
        assembly_timelines: database_record(&package.assembly_timelines)?,
        assembly_clips: database_record(&package.assembly_clips)?,
        source_sync_runs: database_record(&package.source_sync_runs)?,
        source_sync_conflicts: database_record(&package.source_sync_conflicts)?,
        source_sync_decisions: database_record(&package.source_sync_decisions)?,
    })
}

fn database_record<Source, Target>(source: &Source) -> Result<Target, ProjectFamilyImportError>
where
    Source: Serialize,
    Target: DeserializeOwned,
{
    super::super::transfer::transfer_record(source).map_err(|error| {
        ProjectFamilyImportError::invalid_package(format!(
            "项目族清单记录与数据库模型不兼容: {error}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_package_maps_transfer_rows_before_import() {
        let now = chrono::Utc::now().naive_utc();
        let package = ProjectFamilyImportPackage {
            source_snapshots: vec![a4n_domain::project::ProjectFamilySourceSnapshotRecord {
                id: 7,
                root_project_id: 3,
                source_version_hash: "source-hash".into(),
                blueprint_version_id: Some(2),
                chapter_scope_json: serde_json::json!({"start": 1, "end": 2}),
                chapter_hashes_json: serde_json::json!({"1": "hash"}),
                profile_hashes_json: serde_json::json!({}),
                created_at: now,
            }],
            media_assets: vec![a4n_domain::project::ProjectFamilyMediaAssetRecord {
                id: 11,
                project_id: 4,
                root_project_id: 3,
                source_snapshot_id: Some(7),
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: "image".into(),
                origin: "imported".into(),
                file_path: Some("media/image.png".into()),
                thumbnail_path: None,
                width: Some(16),
                height: Some(9),
                duration_secs: None,
                fps: None,
                file_size: Some(12),
                checksum: Some("checksum".into()),
                status: "completed".into(),
                error_message: None,
                created_at: now,
                updated_at: now,
            }],
            authoring_documents: vec![a4n_domain::project::ProjectFamilyAuthoringDocumentRecord {
                id: 19,
                project_id: 4,
                source_snapshot_id: 7,
                source_chapter_id: Some(8),
                kind: "adaptation_plan".into(),
                version: 1,
                revision: 2,
                status: "adopted".into(),
                inputs_json: serde_json::json!({"sources": []}),
                document_json: serde_json::json!({"units": []}),
                document_sha256: "document-hash".into(),
                created_at: now,
                updated_at: now,
            }],
            ..Default::default()
        };

        let database = database_package(&package).expect("transfer rows map to DB rows");
        assert_eq!(
            database.source_snapshots[0].source_version_hash,
            "source-hash"
        );
        assert_eq!(
            database.media_assets[0].file_path.as_deref(),
            Some("media/image.png")
        );
        assert_eq!(
            database.authoring_documents[0].document_json["units"],
            serde_json::json!([])
        );
    }
}
