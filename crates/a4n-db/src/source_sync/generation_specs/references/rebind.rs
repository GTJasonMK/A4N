use super::{
    context::{generation_spec_for_run, update_generation_spec_to_snapshot_record},
    contracts::SourceSyncAnchorMappingRecord,
};
use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_core::models::{
    generation::{generation_spec, generation_spec_text_anchor},
    project::text_anchor,
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, sea_query::Expr};

pub async fn rebind_generation_spec_anchor_records<C>(
    db: &C,
    run: &source_sync_run::Model,
    generation_spec_id: i64,
    anchor_mappings: &[SourceSyncAnchorMappingRecord],
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;
    validate_positive_id(generation_spec_id, "generation_spec_id")?;
    if anchor_mappings.is_empty() {
        return Err(SourceSyncDbError::InvalidArgument(
            "decision_payload_json.anchor_mappings 不能为空".to_string(),
        ));
    }

    let spec = generation_spec_for_run(db, run, generation_spec_id).await?;
    for mapping in anchor_mappings {
        validate_positive_id(mapping.old_text_anchor_id, "old_text_anchor_id")?;
        validate_positive_id(mapping.new_text_anchor_id, "new_text_anchor_id")?;
        validate_new_anchor_for_snapshot(
            db,
            run.root_project_id,
            run.to_snapshot_id,
            mapping.new_text_anchor_id,
        )
        .await?;
        let result = generation_spec_text_anchor::Entity::update_many()
            .col_expr(
                generation_spec_text_anchor::Column::TextAnchorId,
                Expr::value(mapping.new_text_anchor_id),
            )
            .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(generation_spec_id))
            .filter(
                generation_spec_text_anchor::Column::TextAnchorId.eq(mapping.old_text_anchor_id),
            )
            .exec(db)
            .await?;
        if result.rows_affected == 0 {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "生成意图 {} 未引用旧文本锚点 {}",
                generation_spec_id, mapping.old_text_anchor_id
            )));
        }
    }
    ensure_generation_spec_anchors_belong_to_snapshot(db, generation_spec_id, run.to_snapshot_id)
        .await?;
    update_generation_spec_to_snapshot_record(db, &spec, run.to_snapshot_id).await
}

async fn validate_new_anchor_for_snapshot<C>(
    db: &C,
    root_project_id: i64,
    to_snapshot_id: i64,
    new_anchor_id: i64,
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let anchor = text_anchor::Entity::find_by_id(new_anchor_id)
        .one(db)
        .await?
        .ok_or_else(|| SourceSyncDbError::NotFound(format!("文本锚点不存在: {new_anchor_id}")))?;
    if anchor.project_id != root_project_id || anchor.source_snapshot_id != Some(to_snapshot_id) {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "新文本锚点 {} 不属于目标根项目和目标快照",
            new_anchor_id
        )));
    }
    Ok(())
}

async fn ensure_generation_spec_anchors_belong_to_snapshot<C>(
    db: &C,
    generation_spec_id: i64,
    snapshot_id: i64,
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let refs = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(generation_spec_id))
        .all(db)
        .await?;
    if refs.is_empty() {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "生成意图 {} 缺少文本锚点引用",
            generation_spec_id
        )));
    }
    for item in refs {
        let anchor = text_anchor::Entity::find_by_id(item.text_anchor_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                SourceSyncDbError::NotFound(format!("文本锚点不存在: {}", item.text_anchor_id))
            })?;
        if anchor.source_snapshot_id != Some(snapshot_id) {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "生成意图 {} 仍引用非目标快照文本锚点 {}",
                generation_spec_id, anchor.id
            )));
        }
    }
    Ok(())
}
