use super::{ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_required};
use crate::canvas::{
    import_video_studio_canvas_graphs, import_video_studio_canvas_text_assets,
    import_video_studio_generation_specs, update_imported_canvas_node_refs,
    update_imported_generation_spec_metadata, update_imported_generation_spec_targets,
    update_imported_media_metadata,
};
use a4n_db_models::models::generation::generation_spec;
use sea_orm::DatabaseTransaction;

pub(super) async fn insert_canvas_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let now = chrono::Utc::now().naive_utc();
    for canvas in &package.canvas_projects {
        let project_id = map_required(&maps.project_ids, canvas.project.id, "canvas.project_id")?;
        import_video_studio_canvas_text_assets(
            txn,
            project_id,
            &canvas.canvas_text_assets,
            &mut maps.canvas,
            now,
        )
        .await?;
        import_video_studio_canvas_graphs(txn, project_id, canvas, &mut maps.canvas, now).await?;
    }
    Ok(())
}

pub(super) async fn insert_canvas_generation_specs(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let now = chrono::Utc::now().naive_utc();
    for canvas in &package.canvas_projects {
        let project_id = map_required(&maps.project_ids, canvas.project.id, "canvas.project_id")?;
        let specs = package
            .generation_specs
            .iter()
            .filter(|spec| spec.project_id == canvas.project.id && is_canvas_spec(spec))
            .cloned()
            .collect::<Vec<_>>();
        maps.canvas_targets.extend(
            import_video_studio_generation_specs(txn, project_id, &specs, &mut maps.canvas, now)
                .await?,
        );
    }
    for spec in package
        .generation_specs
        .iter()
        .filter(|spec| is_canvas_spec(spec))
    {
        let new_id = map_required(
            &maps.canvas.generation_spec_ids,
            spec.id,
            "canvas generation_spec.id",
        )?;
        maps.generation_spec_ids.insert(spec.id, new_id);
    }
    Ok(())
}

pub(super) async fn finish_canvas_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    if package.canvas_projects.is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now().naive_utc();
    // Media stays owned by the family importer. Canvas references use those
    // exact inserted IDs for the owning video's assets.
    maps.canvas.image_ids.clone_from(&maps.generated_image_ids);
    maps.canvas
        .image_file_paths
        .clone_from(&maps.generated_image_paths);
    maps.canvas.video_ids.clone_from(&maps.generated_video_ids);
    maps.canvas.video_file_paths = maps
        .generated_video_paths
        .iter()
        .map(|(id, path)| (*id, Some(path.clone())))
        .collect();
    maps.canvas.video_thumbnail_paths = maps
        .generated_video_thumbnail_paths
        .iter()
        .map(|(id, path)| (*id, Some(path.clone())))
        .collect();
    maps.canvas
        .generation_spec_ids
        .clone_from(&maps.generation_spec_ids);
    maps.canvas
        .prompt_artifact_ids
        .clone_from(&maps.prompt_artifact_ids);
    maps.canvas
        .media_asset_ids
        .clone_from(&maps.media_asset_ids);
    maps.canvas
        .video_segment_ids
        .clone_from(&maps.video_segment_ids);
    maps.canvas
        .assembly_timeline_ids
        .clone_from(&maps.assembly_timeline_ids);
    maps.canvas
        .assembly_clip_ids
        .clone_from(&maps.assembly_clip_ids);
    update_imported_generation_spec_targets(txn, &maps.canvas_targets, &maps.canvas, now).await?;
    let specs = package
        .generation_specs
        .iter()
        .filter(|spec| is_canvas_spec(spec))
        .cloned()
        .collect::<Vec<_>>();
    update_imported_generation_spec_metadata(txn, &specs, &maps.canvas, now).await?;
    for canvas in &package.canvas_projects {
        update_imported_canvas_node_refs(txn, canvas, &maps.canvas, now).await?;
        let mut metadata_rows = canvas.clone();
        metadata_rows.generated_images = package
            .generated_images
            .iter()
            .filter(|image| {
                image.project_id == canvas.project.id
                    && image.image_type == a4n_domain::image::IMAGE_TYPE_CANVAS
            })
            .cloned()
            .collect();
        metadata_rows.generated_videos = package
            .generated_videos
            .iter()
            .filter(|video| {
                video.project_id == canvas.project.id
                    && video.video_type == a4n_domain::video_workflow::VIDEO_TYPE_CANVAS
            })
            .cloned()
            .collect();
        update_imported_media_metadata(txn, &metadata_rows, &maps.canvas, now).await?;
    }
    Ok(())
}

fn is_canvas_spec(spec: &generation_spec::Model) -> bool {
    spec.source_context_kind.as_deref() == Some(generation_spec::SOURCE_CONTEXT_FREEFORM)
}
