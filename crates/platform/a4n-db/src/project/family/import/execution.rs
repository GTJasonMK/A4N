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
        txn.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT a4n_backfill_galgame_scene_sources($1)",
            [project.id.into()],
        ))
        .await?;
        let invalid_scene = txn.query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT scene.id FROM galgame_scene scene
            JOIN galgame_script game ON game.id = scene.script_id
            JOIN source_snapshot_chapter source ON source.id = scene.source_snapshot_chapter_id
            WHERE game.project_id = $1 AND scene.shooting_scene_identity IS NOT NULL
              AND NOT EXISTS (
                SELECT 1 FROM chapter_shooting_script shooting
                CROSS JOIN LATERAL jsonb_array_elements(shooting.script_json::jsonb -> 'scenes') item(value)
                WHERE shooting.source_snapshot_chapter_id = source.id
                  AND shooting.visual_plan_id = source.visual_plan_id
                  AND shooting.visual_plan_version = source.visual_plan_version
                  AND shooting.visual_plan_sha256 = source.visual_plan_sha256
                  AND shooting.source_content_sha256 = source.content_hash
                  AND item.value ->> 'scene_identity' = scene.shooting_scene_identity
              ) LIMIT 1"#,
            [project.id.into()],
        )).await?;
        if let Some(scene) = invalid_scene {
            let scene_id: i64 = scene.try_get("", "id")?;
            return Err(ProjectFamilyImportError::InvalidPackage(format!(
                "Galgame 场 {scene_id} 的拍摄场景来源不属于导入的冻结剧本"
            )));
        }
    }
    txn.commit()
        .await
        .map_err(ProjectFamilyImportError::Commit)?;
    let imported_projects = imported_projects
        .into_iter()
        .map(project_record_from_model)
        .collect();
    Ok((
        imported_projects,
        ProjectFamilyImportMappings {
            chapter_ids: maps.chapter_ids,
            source_snapshot_ids: maps.source_snapshot_ids,
            source_snapshot_chapter_ids: maps.source_snapshot_chapter_ids,
        },
    ))
}
