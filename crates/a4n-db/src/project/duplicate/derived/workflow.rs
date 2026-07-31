use super::assembly::{
    copy_derived_assembly_clips, copy_derived_assembly_timelines, copy_derived_media_bindings,
    update_derived_generation_spec_segment_targets,
};
use super::generation::{
    copy_derived_generation_spec_entity_refs, copy_derived_generation_spec_media_refs,
    copy_derived_generation_spec_text_anchor_refs, copy_derived_generation_specs,
    copy_derived_prompt_artifacts, copy_derived_referenced_text_anchors,
};
use super::media::{
    copy_derived_media_assets, copy_derived_media_provenance, copy_derived_video_segments,
};
use super::*;

pub(super) async fn copy_derived_workflow_records<C, Copier>(
    db: &C,
    source_project_id: i64,
    new_project: &novel_project::Model,
    root_project_id: i64,
    old_snapshot_id: i64,
    duplicated_snapshot: &DuplicatedDerivedSourceSnapshot,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: ProjectMediaFileCopier,
{
    let copied_specs = copy_derived_generation_specs(
        db,
        source_project_id,
        new_project.id,
        root_project_id,
        duplicated_snapshot,
        now,
    )
    .await?;
    let prompt_id_map =
        copy_derived_prompt_artifacts(db, &copied_specs.spec_ids, &copied_specs.spec_id_map, now)
            .await?;
    let text_anchor_id_map = copy_derived_referenced_text_anchors(
        db,
        root_project_id,
        old_snapshot_id,
        duplicated_snapshot,
        &copied_specs.spec_ids,
        now,
    )
    .await?;
    copy_derived_generation_spec_text_anchor_refs(
        db,
        &copied_specs.spec_ids,
        &copied_specs.spec_id_map,
        &text_anchor_id_map,
    )
    .await?;
    copy_derived_generation_spec_entity_refs(db, &copied_specs.spec_ids, &copied_specs.spec_id_map)
        .await?;
    let asset_id_map = copy_derived_media_assets(
        db,
        source_project_id,
        new_project.id,
        root_project_id,
        duplicated_snapshot,
        now,
        media_file_copier,
    )
    .await?;
    copy_derived_media_provenance(
        db,
        &asset_id_map,
        &copied_specs.spec_id_map,
        &prompt_id_map,
        now,
    )
    .await?;
    let segment_id_map = copy_derived_video_segments(
        db,
        source_project_id,
        new_project.id,
        root_project_id,
        duplicated_snapshot,
        &copied_specs.spec_id_map,
        &asset_id_map,
        now,
    )
    .await?;
    update_derived_generation_spec_segment_targets(
        db,
        &copied_specs.segment_target_updates,
        &segment_id_map,
        now,
    )
    .await?;
    let timeline_id_map = copy_derived_assembly_timelines(
        db,
        source_project_id,
        new_project.id,
        duplicated_snapshot,
        &asset_id_map,
        now,
    )
    .await?;
    let _clip_id_map =
        copy_derived_assembly_clips(db, &timeline_id_map, &asset_id_map, &segment_id_map).await?;
    copy_derived_generation_spec_media_refs(
        db,
        &copied_specs.spec_ids,
        &copied_specs.spec_id_map,
        &asset_id_map,
    )
    .await?;
    copy_derived_media_bindings(
        db,
        source_project_id,
        &asset_id_map,
        &segment_id_map,
        &timeline_id_map,
        None,
        now,
    )
    .await?;
    Ok(())
}
