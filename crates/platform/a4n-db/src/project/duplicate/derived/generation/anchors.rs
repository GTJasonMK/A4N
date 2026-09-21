use std::collections::{HashMap, HashSet};

use a4n_db_models::models::{generation::generation_spec_text_anchor, project::text_anchor};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::shared::remap_derived_optional_id;

use super::super::types::DuplicatedDerivedSourceSnapshot;

pub(in crate::project::duplicate::derived) async fn copy_derived_referenced_text_anchors<C>(
    db: &C,
    root_project_id: i64,
    old_snapshot_id: i64,
    duplicated_snapshot: &DuplicatedDerivedSourceSnapshot,
    old_spec_ids: &[i64],
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if old_spec_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let refs = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let anchor_ids = refs
        .iter()
        .map(|item| item.text_anchor_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if anchor_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let anchors = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(anchor_ids))
        .order_by_asc(text_anchor::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let rows = anchors
        .iter()
        .map(|anchor| {
            if anchor.project_id != root_project_id
                || anchor.source_snapshot_id != Some(old_snapshot_id)
            {
                return Err(ProjectDuplicateError::InvalidState {
                    current: format!("文本锚点 {} 不属于源快照", anchor.id),
                    operation: "复制派生项目生成意图引用".to_string(),
                });
            }
            let source_snapshot_chapter_id = remap_derived_optional_id(
                &duplicated_snapshot.chapter_ids,
                anchor.source_snapshot_chapter_id,
                "text_anchor.source_snapshot_chapter_id",
            )?;
            let source_id =
                source_snapshot_chapter_id.ok_or_else(|| ProjectDuplicateError::InvalidState {
                    current: format!(
                        "源快照文本锚点 {} 缺少 source_snapshot_chapter_id",
                        anchor.id
                    ),
                    operation: "复制派生项目文本锚点".to_string(),
                })?;
            Ok::<_, ProjectDuplicateError>(text_anchor::ActiveModel {
                project_id: Set(anchor.project_id),
                source_chapter_id: Set(anchor.source_chapter_id),
                source_snapshot_id: Set(Some(duplicated_snapshot.snapshot.id)),
                source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
                source_kind: Set(anchor.source_kind.clone()),
                source_id: Set(source_id),
                paragraph_index: Set(anchor.paragraph_index),
                start_offset: Set(anchor.start_offset),
                end_offset: Set(anchor.end_offset),
                quote: Set(anchor.quote.clone()),
                quote_hash: Set(anchor.quote_hash.clone()),
                source_version_hash: Set(duplicated_snapshot.snapshot.source_version_hash.clone()),
                created_at: Set(now),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    if rows.is_empty() {
        return Ok(HashMap::new());
    }
    let inserted = text_anchor::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != anchors.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的文本锚点数量与源记录不一致".to_string(),
            operation: "复制派生项目文本锚点".to_string(),
        });
    }
    let mut anchor_id_map = HashMap::with_capacity(anchors.len());
    for (anchor, copied) in anchors.iter().zip(inserted) {
        anchor_id_map.insert(anchor.id, copied.id);
    }
    Ok(anchor_id_map)
}
