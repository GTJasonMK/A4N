use std::collections::{BTreeMap, BTreeSet};

use a4n_core::models::chapter::part_outline;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::super::{OutlineDbError, PartOutlineSyncDraft};
use super::records::{
    insert_part_outline_record, part_outline_matches, update_part_outline_record,
};

/// 按 part_number 增量同步分部大纲：保留匹配 id、更新变更、插入缺失、删除多余。
pub async fn sync_part_outline_records<C>(
    db: &C,
    project_id: i64,
    drafts: Vec<PartOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<part_outline::Model>, OutlineDbError>
where
    C: ConnectionTrait,
{
    let mut seen = BTreeSet::new();
    for draft in &drafts {
        if !seen.insert(draft.part_number) {
            return Err(OutlineDbError::InvalidArgument(format!(
                "分部大纲重复: 第 {} 部",
                draft.part_number
            )));
        }
    }

    let existing = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await?;
    let mut existing_by_number = BTreeMap::<i32, Vec<part_outline::Model>>::new();
    for part in existing {
        existing_by_number
            .entry(part.part_number)
            .or_default()
            .push(part);
    }

    let mut results = Vec::with_capacity(drafts.len());
    for draft in drafts {
        if let Some(candidates) = existing_by_number.get_mut(&draft.part_number)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if part_outline_matches(&existing, &draft) {
                results.push(existing);
                continue;
            }
            let updated = update_part_outline_record(db, existing, &draft, now).await?;
            results.push(updated);
        } else {
            let inserted = insert_part_outline_record(db, project_id, &draft, now).await?;
            results.push(inserted);
        }
    }

    for stale in existing_by_number.into_values().flatten() {
        part_outline::Entity::delete_by_id(stale.id)
            .exec(db)
            .await?;
    }

    Ok(results)
}
