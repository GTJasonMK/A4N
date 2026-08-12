use std::collections::{BTreeMap, BTreeSet};

use a4n_core::models::character::foreshadowing_index;
use a4n_domain::character::ForeshadowingStatus;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

use super::{
    contracts::{
        ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, ForeshadowingDbError,
    },
    rules::{
        ForeshadowingResolutionAction, foreshadowing_matches_analysis_draft,
        foreshadowing_related_characters_after_setup_resync, foreshadowing_resolution_action,
    },
};
use crate::project::ChapterSourceSnapshot;

pub async fn sync_chapter_analysis_foreshadowing(
    db: &DatabaseConnection,
    source: ChapterSourceSnapshot,
    drafts: Vec<ForeshadowingAnalysisDraft>,
    resolved_ids: Vec<i64>,
) -> Result<ForeshadowingAnalysisSyncResult, ForeshadowingDbError> {
    let txn = db.begin().await?;
    super::records::lock_project_for_write(&txn, source.project_id, "保存章节伏笔分析结果").await?;
    if !crate::chapter_content::chapter_source_snapshots_match_for_update(
        &txn,
        std::slice::from_ref(&source),
    )
    .await?
    {
        return Err(ForeshadowingDbError::InvalidState {
            current: "chapter_source_changed".to_string(),
            operation: "保存章节伏笔同步结果；章节正文已变化，请重新分析".to_string(),
        });
    }
    let result = sync_in(
        &txn,
        source.project_id,
        source.chapter_number,
        drafts,
        resolved_ids,
    )
    .await?;
    txn.commit().await?;
    Ok(result)
}

async fn sync_in<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    drafts: Vec<ForeshadowingAnalysisDraft>,
    resolved_ids: Vec<i64>,
) -> Result<ForeshadowingAnalysisSyncResult, ForeshadowingDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let mut query = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .order_by_asc(foreshadowing_index::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let existing = query.all(db).await?;
    let mut by_id = BTreeMap::<i64, foreshadowing_index::Model>::new();
    let mut setup_by_description = BTreeMap::<String, Vec<foreshadowing_index::Model>>::new();
    for item in existing {
        if item.setup_chapter == chapter_number {
            setup_by_description
                .entry(item.description.clone())
                .or_default()
                .push(item.clone());
        }
        by_id.insert(item.id, item);
    }

    let mut result = ForeshadowingAnalysisSyncResult {
        current: Vec::with_capacity(drafts.len() + resolved_ids.len()),
        ..Default::default()
    };
    for draft in drafts {
        if let Some(candidates) = setup_by_description.get_mut(&draft.description)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if foreshadowing_matches_analysis_draft(&existing, &draft) {
                result.existing.push(existing.clone());
                result.current.push(existing);
                continue;
            }
            let related_characters =
                foreshadowing_related_characters_after_setup_resync(&existing, &draft);
            let updated_at =
                std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
            let mut active: foreshadowing_index::ActiveModel = existing.into();
            active.related_characters = Set(related_characters);
            active.updated_at = Set(updated_at);
            let updated = active.update(db).await?;
            result.updated.push(updated.clone());
            result.current.push(updated);
        } else {
            let created = (foreshadowing_index::ActiveModel {
                project_id: Set(project_id),
                description: Set(draft.description),
                setup_chapter: Set(chapter_number),
                payoff_chapter: Set(None),
                status: Set(ForeshadowingStatus::Pending.as_str().to_string()),
                related_characters: Set(draft.related_characters),
                importance: Set(draft.importance),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .insert(db)
            .await?;
            result.created.push(created.clone());
            result.current.push(created);
        }
    }

    let mut requested = BTreeSet::new();
    for foreshadowing_id in resolved_ids {
        if !requested.insert(foreshadowing_id) {
            return Err(ForeshadowingDbError::DuplicateResolutionTarget {
                foreshadowing_id,
                chapter_number,
            });
        }
    }

    for foreshadowing_id in requested {
        let existing = by_id.get(&foreshadowing_id).ok_or_else(|| {
            ForeshadowingDbError::InvalidResolutionTarget {
                foreshadowing_id,
                chapter_number,
                reason: "项目中不存在该伏笔".to_string(),
            }
        })?;
        match foreshadowing_resolution_action(existing, chapter_number).map_err(|reason| {
            ForeshadowingDbError::InvalidResolutionTarget {
                foreshadowing_id,
                chapter_number,
                reason,
            }
        })? {
            ForeshadowingResolutionAction::Keep => {
                result.existing.push(existing.clone());
                result.current.push(existing.clone());
            }
            ForeshadowingResolutionAction::Resolve => {
                let updated_at =
                    std::cmp::max(now, existing.updated_at + chrono::Duration::microseconds(1));
                let mut active: foreshadowing_index::ActiveModel = existing.clone().into();
                active.payoff_chapter = Set(Some(chapter_number));
                active.status = Set(ForeshadowingStatus::Resolved.as_str().to_string());
                active.updated_at = Set(updated_at);
                let resolved = active.update(db).await?;
                result.resolved.push(resolved.clone());
                result.current.push(resolved);
            }
        }
    }

    Ok(result)
}
