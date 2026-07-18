use a4n_core::models::project::novel_project;
use chrono::NaiveDateTime;
use sea_orm::ConnectionTrait;

use super::{
    FreeformCopyMaps, FreeformProjectMediaCopyResult, FreeformProjectMediaFileCopier,
    canvas::{
        copy_freeform_canvas_graphs_record, copy_freeform_canvas_text_assets_record,
        copy_freeform_generation_specs_record,
        update_freeform_generation_spec_deferred_targets_record,
        update_freeform_generation_spec_metadata_record,
    },
    ensure_freeform_source_mode,
    media::{
        copy_freeform_media_provenance_record, copy_freeform_project_media_graph_record,
        update_freeform_assembly_timeline_cache_keys_record,
        update_freeform_generated_media_metadata_record,
    },
    records::duplicate_freeform_project_media_record,
};
use crate::project::duplicate::{
    ProjectDuplicateError,
    derived::{
        copy_derived_generation_spec_media_refs, copy_derived_media_bindings,
        copy_derived_prompt_artifacts,
    },
};

pub async fn duplicate_freeform_project_record<C, Copier>(
    db: &C,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<novel_project::Model, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: FreeformProjectMediaFileCopier,
{
    ensure_freeform_source_mode(&original.source_mode)?;

    let FreeformProjectMediaCopyResult {
        project: new_project,
        images,
        videos,
    } = duplicate_freeform_project_media_record(
        db,
        original,
        user_id,
        new_title,
        now,
        media_file_copier,
    )
    .await?;

    let mut maps = FreeformCopyMaps {
        images,
        videos,
        ..Default::default()
    };
    maps.text_asset_ids =
        copy_freeform_canvas_text_assets_record(db, original.id, new_project.id, now).await?;

    let copied_specs = copy_freeform_generation_specs_record(
        db,
        original.id,
        new_project.id,
        &maps.text_asset_ids,
        now,
    )
    .await?;
    maps.generation_spec_ids = copied_specs.spec_id_map;
    maps.prompt_artifact_ids =
        copy_derived_prompt_artifacts(db, &copied_specs.spec_ids, &maps.generation_spec_ids, now)
            .await?;

    let copied_media_graph = copy_freeform_project_media_graph_record(
        db,
        original.id,
        new_project.id,
        &maps.generation_spec_ids,
        now,
        media_file_copier,
    )
    .await?;
    maps.media_asset_ids = copied_media_graph.media_asset_ids;
    maps.video_segment_ids = copied_media_graph.video_segment_ids;
    maps.assembly_timeline_ids = copied_media_graph.assembly_timeline_ids;
    maps.assembly_clip_ids = copied_media_graph.assembly_clip_ids;

    let copied_canvas =
        copy_freeform_canvas_graphs_record(db, original.id, new_project.id, &maps, now).await?;
    maps.graph_ids = copied_canvas.graph_ids;
    maps.node_ids = copied_canvas.node_ids;

    update_freeform_generation_spec_deferred_targets_record(
        db,
        &copied_specs.deferred_targets,
        &maps,
        now,
    )
    .await?;
    update_freeform_generation_spec_metadata_record(db, &maps.generation_spec_ids, &maps, now)
        .await?;
    copy_derived_generation_spec_media_refs(
        db,
        &copied_specs.spec_ids,
        &maps.generation_spec_ids,
        &maps.media_asset_ids,
    )
    .await?;
    copy_freeform_media_provenance_record(db, &maps.media_asset_ids, &maps, now).await?;
    copy_derived_media_bindings(
        db,
        original.id,
        &maps.media_asset_ids,
        &maps.video_segment_ids,
        &maps.assembly_timeline_ids,
        Some(&maps.videos.ids),
        now,
    )
    .await?;
    update_freeform_generated_media_metadata_record(db, original.id, &maps, now).await?;
    update_freeform_assembly_timeline_cache_keys_record(
        db,
        &maps.assembly_timeline_ids,
        &maps,
        now,
    )
    .await?;

    Ok(new_project)
}
