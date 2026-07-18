use std::collections::BTreeMap;

use a4n_core::models::character::foreshadowing_index;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

use super::{
    contracts::{
        ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, ForeshadowingDbError,
    },
    rules::{foreshadowing_matches_analysis_draft, foreshadowing_status_after_setup_resync},
};

pub async fn sync_chapter_analysis_foreshadowing<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
    drafts: Vec<ForeshadowingAnalysisDraft>,
) -> Result<ForeshadowingAnalysisSyncResult, ForeshadowingDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let existing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::SetupChapter.eq(chapter_number))
        .order_by_asc(foreshadowing_index::Column::Id)
        .all(db)
        .await
        .map_err(ForeshadowingDbError::Database)?;
    let mut existing_by_description = BTreeMap::<String, Vec<foreshadowing_index::Model>>::new();
    for item in existing {
        existing_by_description
            .entry(item.description.clone())
            .or_default()
            .push(item);
    }

    let mut current = Vec::with_capacity(drafts.len());
    for draft in drafts {
        if let Some(candidates) = existing_by_description.get_mut(&draft.description)
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
            let updated = active
                .update(db)
                .await
                .map_err(ForeshadowingDbError::Database)?;
            current.push(updated);
        } else {
            let inserted_item = (foreshadowing_index::ActiveModel {
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
            .await
            .map_err(ForeshadowingDbError::Database)?;
            current.push(inserted_item);
        }
    }

    let mut stale_ids = Vec::new();
    for stale in existing_by_description.into_values().flatten() {
        stale_ids.push(stale.id);
        foreshadowing_index::Entity::delete_by_id(stale.id)
            .exec(db)
            .await
            .map_err(ForeshadowingDbError::Database)?;
    }

    Ok(ForeshadowingAnalysisSyncResult { current, stale_ids })
}
