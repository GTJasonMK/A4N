use std::collections::{BTreeMap, BTreeSet};

use a4n_core::models::character::foreshadowing_index;
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
        foreshadowing_resolution_action, foreshadowing_status_after_setup_resync,
    },
};

pub async fn sync_chapter_analysis_foreshadowing(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
    drafts: Vec<ForeshadowingAnalysisDraft>,
    resolved_ids: Vec<i64>,
) -> Result<ForeshadowingAnalysisSyncResult, ForeshadowingDbError> {
    let txn = db.begin().await?;
    if crate::project::project_by_id_for_update(&txn, project_id)
        .await?
        .is_none()
    {
        return Err(ForeshadowingDbError::ProjectNotFound { project_id });
    }
    let result = sync_in(&txn, project_id, chapter_number, drafts, resolved_ids).await?;
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

    let mut current = Vec::with_capacity(drafts.len() + resolved_ids.len());
    for draft in drafts {
        if let Some(candidates) = setup_by_description.get_mut(&draft.description)
            && !candidates.is_empty()
        {
            let existing = candidates.remove(0);
            if foreshadowing_matches_analysis_draft(&existing, &draft) {
                current.push(existing);
                continue;
            }
            let status = foreshadowing_status_after_setup_resync(
                existing.status.as_str(),
                existing.payoff_chapter,
                draft.status.as_str(),
            );
            let mut active: foreshadowing_index::ActiveModel = existing.into();
            active.status = Set(status);
            active.related_characters = Set(draft.related_characters);
            active.importance = Set(draft.importance);
            active.updated_at = Set(now);
            current.push(active.update(db).await?);
        } else {
            current.push(
                (foreshadowing_index::ActiveModel {
                    project_id: Set(project_id),
                    description: Set(draft.description),
                    setup_chapter: Set(chapter_number),
                    payoff_chapter: Set(None),
                    status: Set(draft.status),
                    related_characters: Set(draft.related_characters),
                    importance: Set(draft.importance),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                })
                .insert(db)
                .await?,
            );
        }
    }

    let mut stale_ids = Vec::new();
    for stale in setup_by_description.into_values().flatten() {
        stale_ids.push(stale.id);
        foreshadowing_index::Entity::delete_by_id(stale.id)
            .exec(db)
            .await?;
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

    for existing in by_id.values().filter(|item| {
        item.payoff_chapter == Some(chapter_number) && !requested.contains(&item.id)
    }) {
        let mut active: foreshadowing_index::ActiveModel = existing.clone().into();
        active.payoff_chapter = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(now);
        current.push(active.update(db).await?);
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
            ForeshadowingResolutionAction::Keep => current.push(existing.clone()),
            ForeshadowingResolutionAction::Resolve => {
                let mut active: foreshadowing_index::ActiveModel = existing.clone().into();
                active.payoff_chapter = Set(Some(chapter_number));
                active.status = Set("resolved".to_string());
                active.updated_at = Set(now);
                current.push(active.update(db).await?);
            }
        }
    }

    Ok(ForeshadowingAnalysisSyncResult { current, stale_ids })
}
