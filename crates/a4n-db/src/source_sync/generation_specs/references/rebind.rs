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
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    sea_query::{CaseStatement, Expr},
};
use std::collections::{HashMap, HashSet};

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
    let mut new_anchor_ids = HashSet::new();
    let mut old_anchor_ids = HashSet::new();
    for mapping in anchor_mappings {
        validate_positive_id(mapping.old_text_anchor_id, "old_text_anchor_id")?;
        validate_positive_id(mapping.new_text_anchor_id, "new_text_anchor_id")?;
        if !old_anchor_ids.insert(mapping.old_text_anchor_id) {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "旧文本锚点 {} 在映射中重复",
                mapping.old_text_anchor_id
            )));
        }
        new_anchor_ids.insert(mapping.new_text_anchor_id);
    }
    let anchors_by_id = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(new_anchor_ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    let existing_refs = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(generation_spec_id))
        .all(db)
        .await?;
    let existing_anchor_ids = existing_refs
        .iter()
        .map(|reference| reference.text_anchor_id)
        .collect::<HashSet<_>>();
    for mapping in anchor_mappings {
        validate_new_anchor_for_snapshot(
            &anchors_by_id,
            run.root_project_id,
            run.to_snapshot_id,
            mapping.new_text_anchor_id,
        )?;
        if !existing_anchor_ids.contains(&mapping.old_text_anchor_id) {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "生成意图 {} 未引用旧文本锚点 {}",
                generation_spec_id, mapping.old_text_anchor_id
            )));
        }
    }
    let mut text_anchor_id_case = CaseStatement::new();
    for mapping in anchor_mappings {
        text_anchor_id_case = text_anchor_id_case.case(
            generation_spec_text_anchor::Column::TextAnchorId.eq(mapping.old_text_anchor_id),
            Expr::value(mapping.new_text_anchor_id),
        );
    }
    let expected_rows = existing_refs
        .iter()
        .filter(|reference| old_anchor_ids.contains(&reference.text_anchor_id))
        .count() as u64;
    let updated = generation_spec_text_anchor::Entity::update_many()
        .col_expr(
            generation_spec_text_anchor::Column::TextAnchorId,
            text_anchor_id_case
                .finally(Expr::col(generation_spec_text_anchor::Column::TextAnchorId))
                .into(),
        )
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(generation_spec_id))
        .filter(
            generation_spec_text_anchor::Column::TextAnchorId.is_in(old_anchor_ids.iter().copied()),
        )
        .exec(db)
        .await?;
    if updated.rows_affected != expected_rows {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "生成意图 {} 锚点重绑定影响 {} 行，预期 {} 行",
            generation_spec_id, updated.rows_affected, expected_rows
        )));
    }
    ensure_generation_spec_anchors_belong_to_snapshot(db, generation_spec_id, run.to_snapshot_id)
        .await?;
    update_generation_spec_to_snapshot_record(db, &spec, run.to_snapshot_id).await
}

fn validate_new_anchor_for_snapshot(
    anchors_by_id: &HashMap<i64, text_anchor::Model>,
    root_project_id: i64,
    to_snapshot_id: i64,
    new_anchor_id: i64,
) -> Result<(), SourceSyncDbError> {
    let anchor = anchors_by_id
        .get(&new_anchor_id)
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
    let anchor_ids = refs
        .iter()
        .map(|item| item.text_anchor_id)
        .collect::<HashSet<_>>();
    let anchors_by_id = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(anchor_ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    for item in refs {
        let anchor = anchors_by_id.get(&item.text_anchor_id).ok_or_else(|| {
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
