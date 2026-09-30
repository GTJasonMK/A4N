use a4n_domain::project::ProjectRecord;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};

use crate::project::project_record_from_model;

use super::{
    ProjectFamilyImportError, ProjectFamilyImportMappings, ProjectFamilyImportPackage,
    ProjectFamilyMediaFileImporter, import_rows,
};

pub async fn import_project_family_package_rows<Importer>(
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
    user_id: i64,
    package: &ProjectFamilyImportPackage,
    media_importer: &mut Importer,
) -> Result<(Vec<ProjectRecord>, ProjectFamilyImportMappings), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    let txn = db.begin().await?;
    let (imported_projects, maps) =
        import_rows::import_package_rows(user_id, &txn, package, media_importer).await?;
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
