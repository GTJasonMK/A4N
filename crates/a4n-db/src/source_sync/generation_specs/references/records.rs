use super::contracts::{
    SourceSyncEntityRefRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncTextAnchorRefRecordInput,
};
use crate::source_sync::{
    SourceSyncDbError, source_sync_generation_db_error, validate_positive_id,
};
use a4n_db_models::models::{
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor,
    },
    project::text_anchor,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::{HashMap, HashSet};

pub async fn replace_generation_spec_ref_records<C>(
    db: &C,
    spec: &generation_spec::Model,
    snapshot_id: i64,
    text_anchor_refs: Vec<SourceSyncTextAnchorRefRecordInput>,
    entity_refs: Vec<SourceSyncEntityRefRecordInput>,
    media_refs: Vec<SourceSyncMediaRefRecordInput>,
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    generation_spec_text_anchor::Entity::delete_many()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(spec.id))
        .exec(db)
        .await?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(generation_spec_entity_ref::Column::GenerationSpecId.eq(spec.id))
        .exec(db)
        .await?;
    generation_spec_media_ref::Entity::delete_many()
        .filter(generation_spec_media_ref::Column::GenerationSpecId.eq(spec.id))
        .exec(db)
        .await?;
    insert_generation_spec_ref_records(
        db,
        spec,
        snapshot_id,
        text_anchor_refs,
        entity_refs,
        media_refs,
    )
    .await
}

pub async fn insert_generation_spec_ref_records<C>(
    db: &C,
    spec: &generation_spec::Model,
    snapshot_id: i64,
    text_anchor_refs: Vec<SourceSyncTextAnchorRefRecordInput>,
    entity_refs: Vec<SourceSyncEntityRefRecordInput>,
    media_refs: Vec<SourceSyncMediaRefRecordInput>,
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    validate_positive_id(snapshot_id, "snapshot_id")?;
    for item in &text_anchor_refs {
        validate_positive_id(item.text_anchor_id, "text_anchor_id")?;
    }
    let text_anchor_ids = text_anchor_refs
        .iter()
        .map(|item| item.text_anchor_id)
        .collect::<HashSet<_>>();
    let anchors_by_id = if text_anchor_ids.is_empty() {
        HashMap::new()
    } else {
        text_anchor::Entity::find()
            .filter(text_anchor::Column::Id.is_in(text_anchor_ids.iter().copied()))
            .all(db)
            .await?
            .into_iter()
            .map(|anchor| (anchor.id, anchor))
            .collect::<HashMap<_, _>>()
    };
    let mut text_anchor_rows = Vec::with_capacity(text_anchor_refs.len());
    for item in text_anchor_refs {
        let anchor = anchors_by_id.get(&item.text_anchor_id).ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("文本锚点不存在: {}", item.text_anchor_id))
        })?;
        if anchor.project_id != spec.root_project_id
            || anchor.source_snapshot_id != Some(snapshot_id)
        {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "文本锚点 {} 不属于目标根项目和目标快照",
                anchor.id
            )));
        }
        text_anchor_rows.push((anchor.id, item.role));
    }
    crate::generation::insert_generation_text_anchor_refs(db, spec.id, &text_anchor_rows)
        .await
        .map_err(source_sync_generation_db_error)?;
    if !entity_refs.is_empty() {
        let entity_refs_for_validation = entity_refs
            .iter()
            .map(|item| (item.entity_kind.as_str(), item.entity_id))
            .collect::<Vec<_>>();
        crate::generation::validate_entity_refs(
            db,
            spec.root_project_id,
            snapshot_id,
            &entity_refs_for_validation,
        )
        .await
        .map_err(source_sync_generation_db_error)?;
    }
    let entity_refs = entity_refs
        .into_iter()
        .map(|item| (item.entity_kind, item.entity_id, item.role))
        .collect::<Vec<_>>();
    crate::generation::insert_generation_entity_refs(db, spec.id, &entity_refs)
        .await
        .map_err(source_sync_generation_db_error)?;
    let media_refs = media_refs
        .into_iter()
        .map(|item| (item.media_asset_id, item.role))
        .collect::<Vec<_>>();
    crate::generation::insert_generation_media_refs(db, spec, &media_refs)
        .await
        .map_err(source_sync_generation_db_error)?;
    Ok(())
}
