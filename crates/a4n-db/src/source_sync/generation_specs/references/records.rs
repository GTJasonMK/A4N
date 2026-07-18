use super::contracts::{
    SourceSyncEntityRefRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncTextAnchorRefRecordInput,
};
use crate::source_sync::{
    SourceSyncDbError, source_sync_generation_db_error, validate_positive_id,
};
use a4n_core::models::{
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor,
    },
    project::text_anchor,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

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
    for item in text_anchor_refs {
        validate_positive_id(item.text_anchor_id, "text_anchor_id")?;
        let anchor = text_anchor::Entity::find_by_id(item.text_anchor_id)
            .one(db)
            .await?
            .ok_or_else(|| {
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
        crate::generation::insert_generation_text_anchor_ref(db, spec.id, anchor.id, &item.role)
            .await
            .map_err(source_sync_generation_db_error)?;
    }
    for item in entity_refs {
        crate::generation::validate_entity_ref(
            db,
            spec.root_project_id,
            snapshot_id,
            &item.entity_kind,
            item.entity_id,
        )
        .await
        .map_err(source_sync_generation_db_error)?;
        crate::generation::insert_generation_entity_ref(
            db,
            spec.id,
            &item.entity_kind,
            item.entity_id,
            &item.role,
        )
        .await
        .map_err(source_sync_generation_db_error)?;
    }
    for item in media_refs {
        crate::generation::insert_generation_media_ref(db, spec, item.media_asset_id, &item.role)
            .await
            .map_err(source_sync_generation_db_error)?;
    }
    Ok(())
}
