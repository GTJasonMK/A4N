use std::collections::BTreeMap;

use a4n_core::models::{
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::media::{MediaDbError, MediaProvenanceRecordInput};

use super::context::validate_provenance_source_context;

const MEDIA_STATUS_COMPLETED: &str = "completed";

pub(super) async fn validate_media_provenance_refs<C>(
    db: &C,
    asset: &media_asset::Model,
    input: &MediaProvenanceRecordInput,
) -> Result<(), MediaDbError>
where
    C: ConnectionTrait,
{
    let use_row_lock = db.get_database_backend() != DbBackend::Sqlite;
    let referenced_artifact = if let Some(prompt_artifact_id) = input.prompt_artifact_id {
        let mut artifact_query = prompt_artifact::Entity::find_by_id(prompt_artifact_id);
        if use_row_lock {
            artifact_query = artifact_query.lock_exclusive();
        }
        Some(
            artifact_query
                .one(db)
                .await
                .map_err(MediaDbError::Database)?
                .ok_or_else(|| {
                    MediaDbError::NotFound(format!("提示词产物不存在: {prompt_artifact_id}"))
                })?,
        )
    } else {
        None
    };

    let mut generation_spec_ids = input.generation_spec_id.into_iter().collect::<Vec<_>>();
    if let Some(artifact) = referenced_artifact.as_ref() {
        generation_spec_ids.push(artifact.generation_spec_id);
    }
    generation_spec_ids.sort_unstable();
    generation_spec_ids.dedup();
    let generation_specs = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        let mut specs_query = generation_spec::Entity::find()
            .filter(generation_spec::Column::Id.is_in(generation_spec_ids))
            .order_by_asc(generation_spec::Column::Id);
        if use_row_lock {
            specs_query = specs_query.lock_exclusive();
        }
        specs_query.all(db).await.map_err(MediaDbError::Database)?
    };
    let specs_by_id = generation_specs
        .into_iter()
        .map(|spec| (spec.id, spec))
        .collect::<BTreeMap<_, _>>();

    let referenced_spec = if let Some(generation_spec_id) = input.generation_spec_id {
        let spec = specs_by_id.get(&generation_spec_id).ok_or_else(|| {
            MediaDbError::NotFound(format!("生成意图不存在: {generation_spec_id}"))
        })?;
        if spec.project_id != asset.project_id || spec.root_project_id != asset.root_project_id {
            return Err(MediaDbError::InvalidArgument(format!(
                "媒体来源生成意图 {} 不属于媒体资产项目族",
                spec.id
            )));
        }
        validate_provenance_source_context(
            asset,
            spec.source_snapshot_id,
            spec.source_snapshot_chapter_id,
            spec.source_chapter_id,
            "生成意图",
            spec.id,
        )?;
        Some(spec)
    } else {
        None
    };

    if let Some(artifact) = referenced_artifact.as_ref() {
        let prompt_spec = specs_by_id
            .get(&artifact.generation_spec_id)
            .ok_or_else(|| {
                MediaDbError::NotFound(format!("生成意图不存在: {}", artifact.generation_spec_id))
            })?;
        if prompt_spec.project_id != asset.project_id
            || prompt_spec.root_project_id != asset.root_project_id
        {
            return Err(MediaDbError::InvalidArgument(format!(
                "提示词产物 {} 不属于媒体资产项目族",
                artifact.id
            )));
        }
        validate_provenance_source_context(
            asset,
            prompt_spec.source_snapshot_id,
            prompt_spec.source_snapshot_chapter_id,
            prompt_spec.source_chapter_id,
            "提示词产物生成意图",
            prompt_spec.id,
        )?;
        if let Some(spec) = referenced_spec
            && artifact.generation_spec_id != spec.id
        {
            return Err(MediaDbError::InvalidArgument(format!(
                "提示词产物 {} 不属于生成意图 {}",
                artifact.id, spec.id
            )));
        }
    }

    if let Some(source_media_asset_id) = input.source_media_asset_id {
        let mut source_query = media_asset::Entity::find_by_id(source_media_asset_id);
        if use_row_lock {
            source_query = source_query.lock_exclusive();
        }
        let source = source_query
            .one(db)
            .await
            .map_err(MediaDbError::Database)?
            .ok_or_else(|| {
                MediaDbError::NotFound(format!("源媒体资产不存在: {source_media_asset_id}"))
            })?;
        if source.project_id != asset.project_id || source.root_project_id != asset.root_project_id
        {
            return Err(MediaDbError::InvalidArgument(format!(
                "源媒体资产 {} 不属于媒体资产项目族",
                source.id
            )));
        }
        validate_provenance_source_context(
            asset,
            source.source_snapshot_id,
            source.source_snapshot_chapter_id,
            source.source_chapter_id,
            "源媒体资产",
            source.id,
        )?;
        if source.status != MEDIA_STATUS_COMPLETED || source.file_path.is_none() {
            return Err(MediaDbError::InvalidArgument(format!(
                "源媒体资产 {} 不是可追溯的已完成媒体",
                source.id
            )));
        }
    }
    Ok(())
}
