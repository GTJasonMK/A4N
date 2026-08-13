use super::super::import_canvas::{
    import_video_studio_canvas_graphs, import_video_studio_canvas_text_assets,
    import_video_studio_generation_specs, import_video_studio_prompt_artifacts,
    update_imported_generation_spec_metadata, update_imported_generation_spec_targets,
};
use super::super::import_media::{
    import_video_studio_assembly_rows, import_video_studio_media_assets,
    import_video_studio_media_bindings, import_video_studio_media_provenance,
    import_video_studio_video_segments, update_imported_canvas_node_refs,
    update_imported_media_metadata,
};
use super::super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportIdMaps,
    VideoStudioProjectImportMediaFileWriter, VideoStudioProjectImportRows,
};
use super::VideoStudioProjectImportMediaRowsContext;
use super::generated_media::{
    import_video_studio_generated_images, import_video_studio_generated_videos,
    update_imported_video_source_refs,
};
use super::project::insert_video_studio_project_record;
use a4n_core::models::project::novel_project;
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

pub async fn import_video_studio_project_record<Writer>(
    db: &DatabaseConnection,
    user_id: i64,
    rows: VideoStudioProjectImportRows,
    now: chrono::NaiveDateTime,
    media_file_writer: &mut Writer,
) -> Result<novel_project::Model, VideoStudioProjectImportError>
where
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    let txn = db.begin().await?;
    let project =
        import_video_studio_project_record_rows(&txn, user_id, rows, now, media_file_writer)
            .await?;
    txn.commit()
        .await
        .map_err(VideoStudioProjectImportError::TransactionCommit)?;
    Ok(project)
}

async fn import_video_studio_project_record_rows<C, Writer>(
    db: &C,
    user_id: i64,
    rows: VideoStudioProjectImportRows,
    now: chrono::NaiveDateTime,
    media_file_writer: &mut Writer,
) -> Result<novel_project::Model, VideoStudioProjectImportError>
where
    C: ConnectionTrait,
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    let project = insert_video_studio_project_record(db, user_id, &rows.project, now).await?;
    let mut maps = VideoStudioProjectImportIdMaps::default();

    let imported_videos = {
        let mut media_context = VideoStudioProjectImportMediaRowsContext {
            db,
            project_id: project.id,
            maps: &mut maps,
            media_file_writer,
            now,
        };
        import_video_studio_generated_images(&mut media_context, &rows.generated_images).await?;
        import_video_studio_generated_videos(&mut media_context, &rows.generated_videos).await?
    };
    update_imported_video_source_refs(db, &rows.generated_videos, &imported_videos, &maps, now)
        .await?;
    import_video_studio_canvas_text_assets(
        db,
        project.id,
        &rows.canvas_text_assets,
        &mut maps,
        now,
    )
    .await?;
    import_video_studio_canvas_graphs(db, project.id, &rows, &mut maps, now).await?;
    let deferred_generation_spec_targets = import_video_studio_generation_specs(
        db,
        project.id,
        &rows.generation_specs,
        &mut maps,
        now,
    )
    .await?;
    import_video_studio_prompt_artifacts(db, &rows.prompt_artifacts, &mut maps, now).await?;
    let mut media_context = VideoStudioProjectImportMediaRowsContext {
        db,
        project_id: project.id,
        maps: &mut maps,
        media_file_writer,
        now,
    };
    import_video_studio_media_assets(&mut media_context, &rows.media_assets).await?;
    import_video_studio_video_segments(db, project.id, &rows.video_segments, &mut maps, now)
        .await?;
    import_video_studio_assembly_rows(db, project.id, &rows, &mut maps, now).await?;
    update_imported_generation_spec_targets(db, &deferred_generation_spec_targets, &maps, now)
        .await?;
    update_imported_generation_spec_metadata(db, &rows.generation_specs, &maps, now).await?;
    import_video_studio_media_provenance(db, &rows.media_provenance, &maps, now).await?;
    import_video_studio_media_bindings(db, &rows.media_bindings, &mut maps, now).await?;
    update_imported_canvas_node_refs(db, &rows, &maps, now).await?;
    update_imported_media_metadata(db, &rows, &maps, now).await?;

    Ok(project)
}
