use super::super::remap::{
    initial_video_studio_generation_spec_target_entity_id,
    remap_video_studio_generation_spec_target_entity_id_parts, remap_video_studio_required_id,
};
use super::super::types::{
    DeferredVideoStudioImportGenerationSpecTarget, VideoStudioProjectImportError,
    VideoStudioProjectImportIdMaps,
};
use a4n_core::models::generation::{generation_spec, prompt_artifact};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub(in crate::canvas::transfer) async fn import_video_studio_generation_specs<C>(
    db: &C,
    project_id: i64,
    specs: &[generation_spec::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<Vec<DeferredVideoStudioImportGenerationSpecTarget>, VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    let mut deferred_targets = Vec::new();
    for spec in specs {
        let canvas_text_asset_id = spec
            .canvas_text_asset_id
            .map(|old_id| {
                remap_video_studio_required_id(
                    &maps.text_asset_ids,
                    old_id,
                    "generation_spec.canvas_text_asset_id",
                )
            })
            .transpose()?;
        let target = initial_video_studio_generation_spec_target_entity_id(spec, maps)?;
        let row = generation_spec::ActiveModel {
            project_id: Set(project_id),
            root_project_id: Set(project_id),
            source_context_kind: Set(Some(
                generation_spec::SOURCE_CONTEXT_FREEFORM.to_string(),
            )),
            source_snapshot_id: Set(None),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            canvas_text_asset_id: Set(canvas_text_asset_id),
            target_kind: Set(spec.target_kind.clone()),
            target_entity_kind: Set(spec.target_entity_kind.clone()),
            target_entity_id: Set(target.target_entity_id),
            status: Set(spec.status.clone()),
            source_version_hash: Set(spec.source_version_hash.clone()),
            params_json: Set(spec.params_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        if let Some(deferred) = target.deferred {
            deferred_targets.push(DeferredVideoStudioImportGenerationSpecTarget {
                new_spec_id: row.id,
                target_entity_kind: deferred.target_entity_kind,
                old_target_entity_id: deferred.old_target_entity_id,
            });
        }
        maps.generation_spec_ids.insert(spec.id, row.id);
    }
    Ok(deferred_targets)
}

pub(in crate::canvas::transfer) async fn update_imported_generation_spec_metadata<C>(
    db: &C,
    specs: &[generation_spec::Model],
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for spec in specs {
        let new_id = remap_video_studio_required_id(
            &maps.generation_spec_ids,
            spec.id,
            "generation_spec.id",
        )?;
        let params_json = super::super::remap::remap_video_studio_canvas_metadata(
            spec.params_json.clone(),
            maps,
        )?;
        let source_version_hash = generation_spec::freeform_source_version_hash(&params_json)
            .ok_or_else(|| {
                VideoStudioProjectImportError::InvalidPackage(format!(
                    "生成意图 params_json.source_material 必须是 JSON 对象: generation_spec_id={}",
                    spec.id
                ))
            })?;
        generation_spec::ActiveModel {
            id: Set(new_id),
            source_version_hash: Set(source_version_hash),
            params_json: Set(params_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}

pub(in crate::canvas::transfer) async fn import_video_studio_prompt_artifacts<C>(
    db: &C,
    artifacts: &[prompt_artifact::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for artifact in artifacts {
        let generation_spec_id = remap_video_studio_required_id(
            &maps.generation_spec_ids,
            artifact.generation_spec_id,
            "prompt_artifact.generation_spec_id",
        )?;
        let row = prompt_artifact::ActiveModel {
            generation_spec_id: Set(generation_spec_id),
            prompt_kind: Set(artifact.prompt_kind.clone()),
            prompt_text: Set(artifact.prompt_text.clone()),
            renderer_version: Set(artifact.renderer_version.clone()),
            edited_by_user: Set(artifact.edited_by_user),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
        maps.prompt_artifact_ids.insert(artifact.id, row.id);
    }
    Ok(())
}

pub(in crate::canvas::transfer) async fn update_imported_generation_spec_targets<C>(
    db: &C,
    targets: &[DeferredVideoStudioImportGenerationSpecTarget],
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    for target in targets {
        let target_entity_id = remap_video_studio_generation_spec_target_entity_id_parts(
            target.new_spec_id,
            &target.target_entity_kind,
            target.old_target_entity_id,
            maps,
        )?;
        generation_spec::ActiveModel {
            id: Set(target.new_spec_id),
            target_entity_id: Set(Some(target_entity_id)),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    }
    Ok(())
}
