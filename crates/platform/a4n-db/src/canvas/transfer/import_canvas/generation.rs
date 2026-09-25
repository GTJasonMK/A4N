use super::super::remap::{
    initial_video_studio_generation_spec_target_entity_id,
    remap_video_studio_generation_spec_target_entity_id_parts, remap_video_studio_required_id,
};
use super::super::types::{
    DeferredVideoStudioImportGenerationSpecTarget, VideoStudioProjectImportError,
    VideoStudioProjectImportIdMaps,
};
use a4n_db_models::models::generation::{generation_spec, prompt_artifact};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set,
    sea_query::{Alias, CaseStatement, Expr},
};

pub(crate) async fn import_video_studio_generation_specs<C>(
    db: &C,
    project_id: i64,
    specs: &[generation_spec::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<Vec<DeferredVideoStudioImportGenerationSpecTarget>, VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let mut targets = Vec::with_capacity(specs.len());
    let rows = specs
        .iter()
        .map(|spec| {
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
            let target_entity_id = target.target_entity_id;
            targets.push(target);
            Ok::<_, VideoStudioProjectImportError>(generation_spec::ActiveModel {
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
                target_entity_id: Set(target_entity_id),
                status: Set(spec.status.clone()),
                source_version_hash: Set(spec.source_version_hash.clone()),
                params_json: Set(spec.params_json.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inserted = generation_spec::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if inserted.len() != specs.len() {
        return Err(VideoStudioProjectImportError::InvalidPackage(
            "生成意图批量写入返回行数不一致".to_string(),
        ));
    }
    let mut deferred_targets = Vec::new();
    for ((spec, target), row) in specs.iter().zip(targets).zip(inserted) {
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

pub(crate) async fn update_imported_generation_spec_metadata<C>(
    db: &C,
    specs: &[generation_spec::Model],
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if specs.is_empty() {
        return Ok(());
    }
    let mut source_version_hash_case = CaseStatement::new();
    let mut params_json_case = CaseStatement::new();
    let mut spec_ids = Vec::with_capacity(specs.len());
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
        source_version_hash_case = source_version_hash_case.case(
            generation_spec::Column::Id.eq(new_id),
            Expr::value(source_version_hash),
        );
        params_json_case = params_json_case.case(
            generation_spec::Column::Id.eq(new_id),
            Expr::value(params_json).cast_as(Alias::new("json")),
        );
        spec_ids.push(new_id);
    }
    let updated = generation_spec::Entity::update_many()
        .col_expr(
            generation_spec::Column::SourceVersionHash,
            source_version_hash_case
                .finally(Expr::col(generation_spec::Column::SourceVersionHash))
                .into(),
        )
        .col_expr(
            generation_spec::Column::ParamsJson,
            params_json_case
                .finally(Expr::col(generation_spec::Column::ParamsJson))
                .into(),
        )
        .col_expr(generation_spec::Column::UpdatedAt, Expr::value(now))
        .filter(generation_spec::Column::Id.is_in(spec_ids))
        .exec(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if updated.rows_affected != specs.len() as u64 {
        return Err(VideoStudioProjectImportError::InvalidPackage(
            "生成意图元数据批量回填返回行数不一致".to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn import_video_studio_prompt_artifacts<C>(
    db: &C,
    artifacts: &[prompt_artifact::Model],
    maps: &mut VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if artifacts.is_empty() {
        return Ok(());
    }
    let rows = artifacts
        .iter()
        .map(|artifact| {
            let generation_spec_id = remap_video_studio_required_id(
                &maps.generation_spec_ids,
                artifact.generation_spec_id,
                "prompt_artifact.generation_spec_id",
            )?;
            Ok::<_, VideoStudioProjectImportError>(prompt_artifact::ActiveModel {
                generation_spec_id: Set(generation_spec_id),
                prompt_kind: Set(artifact.prompt_kind.clone()),
                prompt_text: Set(artifact.prompt_text.clone()),
                renderer_version: Set(artifact.renderer_version.clone()),
                edited_by_user: Set(artifact.edited_by_user),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inserted = prompt_artifact::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if inserted.len() != artifacts.len() {
        return Err(VideoStudioProjectImportError::InvalidPackage(
            "PromptArtifact 批量写入返回行数不一致".to_string(),
        ));
    }
    for (artifact, row) in artifacts.iter().zip(inserted) {
        maps.prompt_artifact_ids.insert(artifact.id, row.id);
    }
    Ok(())
}

pub(crate) async fn update_imported_generation_spec_targets<C>(
    db: &C,
    targets: &[DeferredVideoStudioImportGenerationSpecTarget],
    maps: &VideoStudioProjectImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), VideoStudioProjectImportError>
where
    C: ConnectionTrait,
{
    if targets.is_empty() {
        return Ok(());
    }
    let mut target_entity_id_case = CaseStatement::new();
    let mut spec_ids = Vec::with_capacity(targets.len());
    for target in targets {
        let target_entity_id = remap_video_studio_generation_spec_target_entity_id_parts(
            target.new_spec_id,
            &target.target_entity_kind,
            target.old_target_entity_id,
            maps,
        )?;
        target_entity_id_case = target_entity_id_case.case(
            generation_spec::Column::Id.eq(target.new_spec_id),
            Expr::value(target_entity_id),
        );
        spec_ids.push(target.new_spec_id);
    }
    let updated = generation_spec::Entity::update_many()
        .col_expr(
            generation_spec::Column::TargetEntityId,
            target_entity_id_case
                .finally(Expr::col(generation_spec::Column::TargetEntityId))
                .into(),
        )
        .col_expr(generation_spec::Column::UpdatedAt, Expr::value(now))
        .filter(generation_spec::Column::Id.is_in(spec_ids))
        .exec(db)
        .await
        .map_err(VideoStudioProjectImportError::Database)?;
    if updated.rows_affected != targets.len() as u64 {
        return Err(VideoStudioProjectImportError::InvalidPackage(
            "生成意图目标批量回填返回行数不一致".to_string(),
        ));
    }
    Ok(())
}
