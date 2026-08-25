use a4n_core::models::character::foreshadowing_index;
use a4n_domain::character::ForeshadowingStatus;
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::collections::{BTreeSet, HashMap, HashSet};

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
    let mut by_id = HashMap::<i64, foreshadowing_index::Model>::new();
    let mut setup_by_description = HashMap::<String, Vec<foreshadowing_index::Model>>::new();
    for item in existing {
        if item.setup_chapter == chapter_number {
            setup_by_description
                .entry(item.description.clone())
                .or_default()
                .push(item.clone());
        }
        by_id.insert(item.id, item);
    }
    let mut state_by_id = by_id
        .iter()
        .map(|(id, item)| (*id, item.clone()))
        .collect::<HashMap<_, _>>();

    let mut result = ForeshadowingAnalysisSyncResult {
        current: Vec::with_capacity(drafts.len() + resolved_ids.len()),
        ..Default::default()
    };
    let mut draft_update_ids = HashSet::new();
    let mut pending_creates = Vec::new();
    let mut pending_create_slots = Vec::new();
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
            let updated = state_by_id
                .get_mut(&existing.id)
                .expect("existing foreshadowing state must exist");
            updated.related_characters = related_characters;
            updated.updated_at = updated_at;
            draft_update_ids.insert(existing.id);
            result.updated.push(updated.clone());
            result.current.push(updated.clone());
        } else {
            let created = foreshadowing_index::Model {
                id: 0,
                project_id,
                description: draft.description.clone(),
                setup_chapter: chapter_number,
                payoff_chapter: None,
                status: ForeshadowingStatus::Pending.as_str().to_string(),
                related_characters: draft.related_characters.clone(),
                importance: draft.importance,
                created_at: now,
                updated_at: now,
            };
            let current_slot = result.current.len();
            let created_slot = result.created.len();
            result.current.push(created.clone());
            result.created.push(created);
            pending_create_slots.push((current_slot, created_slot));
            pending_creates.push(foreshadowing_index::ActiveModel {
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
            });
        }
    }

    if !pending_creates.is_empty() {
        let expected = pending_creates.len();
        let created = foreshadowing_index::Entity::insert_many(pending_creates)
            .exec_with_returning_many(db)
            .await?;
        if created.len() != expected {
            return Err(ForeshadowingDbError::InvalidArgument(format!(
                "伏笔批量插入返回行数不一致: inserted={}, expected={}",
                created.len(),
                expected
            )));
        }
        for (created, (current_slot, created_slot)) in created.into_iter().zip(pending_create_slots)
        {
            state_by_id.insert(created.id, created.clone());
            result.created[created_slot] = created.clone();
            result.current[current_slot] = created;
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

    let mut resolve_update_ids = HashSet::new();
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
                let resolved = state_by_id
                    .get_mut(&foreshadowing_id)
                    .expect("existing foreshadowing state must exist");
                resolved.payoff_chapter = Some(chapter_number);
                resolved.status = ForeshadowingStatus::Resolved.as_str().to_string();
                resolved.updated_at = updated_at;
                resolve_update_ids.insert(foreshadowing_id);
                result.resolved.push(resolved.clone());
                result.current.push(resolved.clone());
            }
        }
    }

    let mut update_ids = draft_update_ids.clone();
    update_ids.extend(resolve_update_ids.iter().copied());
    if !update_ids.is_empty() {
        let ids = update_ids.into_iter().collect::<Vec<_>>();
        let mut update = foreshadowing_index::Entity::update_many();
        if !draft_update_ids.is_empty() {
            let mut related_characters_case = CaseStatement::new();
            for id in &draft_update_ids {
                let model = state_by_id
                    .get(id)
                    .expect("draft-updated foreshadowing state must exist");
                related_characters_case = related_characters_case.case(
                    foreshadowing_index::Column::Id.eq(*id),
                    Expr::value(model.related_characters.clone()).cast_as("json"),
                );
            }
            update = update.col_expr(
                foreshadowing_index::Column::RelatedCharacters,
                related_characters_case
                    .finally(Expr::col(foreshadowing_index::Column::RelatedCharacters))
                    .into(),
            );
        }
        if !resolve_update_ids.is_empty() {
            let mut payoff_chapter_case = CaseStatement::new();
            let mut status_case = CaseStatement::new();
            for id in &resolve_update_ids {
                let model = state_by_id
                    .get(id)
                    .expect("resolved foreshadowing state must exist");
                payoff_chapter_case = payoff_chapter_case.case(
                    foreshadowing_index::Column::Id.eq(*id),
                    Expr::value(model.payoff_chapter),
                );
                status_case = status_case.case(
                    foreshadowing_index::Column::Id.eq(*id),
                    Expr::value(model.status.clone()),
                );
            }
            update = update
                .col_expr(
                    foreshadowing_index::Column::PayoffChapter,
                    payoff_chapter_case
                        .finally(Expr::col(foreshadowing_index::Column::PayoffChapter))
                        .into(),
                )
                .col_expr(
                    foreshadowing_index::Column::Status,
                    status_case
                        .finally(Expr::col(foreshadowing_index::Column::Status))
                        .into(),
                );
        }
        let mut updated_at_case = CaseStatement::new();
        for id in &ids {
            let model = state_by_id
                .get(id)
                .expect("updated foreshadowing state must exist");
            updated_at_case = updated_at_case.case(
                foreshadowing_index::Column::Id.eq(*id),
                Expr::value(model.updated_at),
            );
        }
        let updated = update
            .col_expr(
                foreshadowing_index::Column::UpdatedAt,
                updated_at_case
                    .finally(Expr::col(foreshadowing_index::Column::UpdatedAt))
                    .into(),
            )
            .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
            .filter(foreshadowing_index::Column::Id.is_in(ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != ids.len() as u64 {
            return Err(ForeshadowingDbError::InvalidArgument(format!(
                "伏笔批量更新返回行数不一致: updated={}, expected={}",
                updated.rows_affected,
                ids.len()
            )));
        }
    }

    Ok(result)
}
