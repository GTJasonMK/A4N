use a4n_db_models::models::{
    chapter::chapter, character::foreshadowing_index, project::material_fact_change,
};
use a4n_domain::character::ForeshadowingStatus;
use chrono::Utc;
use sea_orm::ConnectionTrait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

use super::{
    ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult, ForeshadowingDbError,
    ForeshadowingFactUpdate, foreshadowing_index_record,
};
use a4n_domain::chapter::ChapterSourceSnapshot;

pub async fn sync_chapter_analysis_foreshadowing(
    db: &impl crate::DbExecutor,
    source: ChapterSourceSnapshot,
    drafts: Vec<ForeshadowingAnalysisDraft>,
    updates: Vec<ForeshadowingFactUpdate>,
) -> Result<ForeshadowingAnalysisSyncResult, ForeshadowingDbError> {
    let tx = db.begin().await?;
    super::records::lock_project_for_write(&tx, source.project_id, "保存章节伏笔事实").await?;
    if !crate::chapter_content::chapter_source_snapshots_match_for_update(
        &tx,
        std::slice::from_ref(&source),
    )
    .await?
    {
        return Err(ForeshadowingDbError::InvalidState {
            current: "chapter_source_changed".into(),
            operation: "章节已变化，请重新分析伏笔".into(),
        });
    }
    let chapter = chapter::Entity::find_by_id(source.chapter_id)
        .one(&tx)
        .await?
        .ok_or_else(|| ForeshadowingDbError::InvalidArgument("章节不存在".into()))?;
    let existing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(source.project_id))
        .lock_exclusive()
        .all(&tx)
        .await?;
    let history = material_fact_change::Entity::find()
        .filter(material_fact_change::Column::ProjectId.eq(source.project_id))
        .filter(material_fact_change::Column::MaterialKind.eq("foreshadowing"))
        .filter(material_fact_change::Column::ChapterId.eq(source.chapter_id))
        .order_by_desc(material_fact_change::Column::Id)
        .all(&tx)
        .await?;
    let mut by_id: BTreeMap<_, _> = existing.into_iter().map(|item| (item.id, item)).collect();
    let mut result = ForeshadowingAnalysisSyncResult::default();
    let mut touched = BTreeSet::new();
    // ponytail: a chapter has few foreshadowings; batch writes if large extracts become measurable.
    for draft in drafts {
        require_evidence(&chapter, std::slice::from_ref(&draft.evidence))?;
        if !(1..=5).contains(&draft.importance) {
            return Err(ForeshadowingDbError::InvalidArgument(
                "伏笔重要度必须在 1 到 5 之间".into(),
            ));
        }
        let id = resolve_draft_identity(&draft, source.chapter_number, &by_id, &history)?;
        let prior = id.and_then(|id| by_id.get(&id)).cloned();
        let withdrawn_origin = id.and_then(|id| {
            history.iter().find(|record| {
                record.target_id == Some(id)
                    && record.status == "retracted"
                    && record.before_json.is_null()
            })
        });
        if id.is_some() && prior.is_none() && withdrawn_origin.is_none() {
            return Err(ForeshadowingDbError::InvalidArgument(
                "伏笔 ID 不属于当前项目".into(),
            ));
        }
        if prior
            .as_ref()
            .is_some_and(|item| item.setup_chapter > source.chapter_number)
        {
            return Err(ForeshadowingDbError::InvalidArgument(
                "不能用本章事实修改未来伏笔".into(),
            ));
        }
        let before = prior
            .as_ref()
            .map(foreshadowing_fact_snapshot)
            .unwrap_or(Value::Null);
        let unchanged = prior.as_ref().filter(|item| {
            item.description == draft.description
                && item.related_characters == draft.related_characters
                && item.importance == draft.importance
        });
        let mut active =
            prior
                .clone()
                .map(Into::into)
                .unwrap_or_else(|| foreshadowing_index::ActiveModel {
                    project_id: Set(source.project_id),
                    setup_chapter: Set(source.chapter_number),
                    payoff_chapter: Set(None),
                    status: Set(ForeshadowingStatus::Pending.as_str().into()),
                    // Reanalysis may have just withdrawn the old observation. Its identity is
                    // retained by the source journal, so restore that ID instead of inventing one.
                    id: id.map(Set).unwrap_or(sea_orm::NotSet),
                    created_at: Set(withdrawn_origin
                        .map(|record| record.created_at)
                        .unwrap_or_else(|| Utc::now().naive_utc())),
                    ..Default::default()
                });
        active.description = Set(draft.description);
        active.related_characters = Set(draft.related_characters);
        active.importance = Set(draft.importance);
        active.updated_at = Set(Utc::now().naive_utc());
        let item = if let Some(unchanged) = unchanged {
            unchanged.clone()
        } else if prior.is_some() {
            active.update(&tx).await?
        } else {
            active.insert(&tx).await?
        };
        if !touched.insert(item.id) {
            return Err(ForeshadowingDbError::InvalidArgument(
                "本章重复更新同一伏笔".into(),
            ));
        }
        if before == foreshadowing_fact_snapshot(&item) {
            result
                .existing
                .push(foreshadowing_index_record(item.clone()));
        } else {
            record_change(
                &tx,
                &chapter,
                &item,
                before,
                vec![draft.evidence],
                "正文更新伏笔内容",
            )
            .await?;
            if prior.is_some() {
                result
                    .updated
                    .push(foreshadowing_index_record(item.clone()));
            } else {
                result
                    .created
                    .push(foreshadowing_index_record(item.clone()));
            }
        }
        by_id.insert(item.id, item.clone());
        result.current.push(foreshadowing_index_record(item));
    }
    let mut updated = BTreeSet::new();
    for update in updates {
        require_evidence(&chapter, &update.evidence)?;
        if !updated.insert(update.id) {
            return Err(ForeshadowingDbError::DuplicateResolutionTarget {
                foreshadowing_id: update.id,
                chapter_number: source.chapter_number,
            });
        }
        let prior = by_id.get(&update.id).cloned().ok_or_else(|| {
            ForeshadowingDbError::InvalidResolutionTarget {
                foreshadowing_id: update.id,
                chapter_number: source.chapter_number,
                reason: "项目中不存在该伏笔".into(),
            }
        })?;
        if prior.setup_chapter >= source.chapter_number
            || prior
                .payoff_chapter
                .is_some_and(|chapter| chapter > source.chapter_number)
        {
            return Err(ForeshadowingDbError::InvalidResolutionTarget {
                foreshadowing_id: update.id,
                chapter_number: source.chapter_number,
                reason: "伏笔不在本章可更新范围内".into(),
            });
        }
        if !matches!(
            update.status,
            ForeshadowingStatus::PartiallyRevealed
                | ForeshadowingStatus::Resolved
                | ForeshadowingStatus::Abandoned
        ) {
            return Err(ForeshadowingDbError::InvalidArgument(
                "正文只能部分揭示、回收或明确否定已有伏笔".into(),
            ));
        }
        if update.reason.trim().is_empty() {
            return Err(ForeshadowingDbError::InvalidArgument(
                "伏笔进展缺少变化说明".into(),
            ));
        }
        if prior.status == update.status.as_str() {
            result
                .existing
                .push(foreshadowing_index_record(prior.clone()));
            result.current.push(foreshadowing_index_record(prior));
            continue;
        }
        let before = foreshadowing_fact_snapshot(&prior);
        let mut active: foreshadowing_index::ActiveModel = prior.into();
        active.status = Set(update.status.as_str().into());
        active.payoff_chapter =
            Set((update.status == ForeshadowingStatus::Resolved).then_some(source.chapter_number));
        active.updated_at = Set(Utc::now().naive_utc());
        let item = active.update(&tx).await?;
        if before == foreshadowing_fact_snapshot(&item) {
            result
                .existing
                .push(foreshadowing_index_record(item.clone()));
        } else {
            record_change(
                &tx,
                &chapter,
                &item,
                before,
                update.evidence,
                &update.reason,
            )
            .await?;
            if update.status == ForeshadowingStatus::Resolved {
                result
                    .resolved
                    .push(foreshadowing_index_record(item.clone()));
            } else {
                result
                    .updated
                    .push(foreshadowing_index_record(item.clone()));
            }
        }
        by_id.insert(item.id, item.clone());
        result.current.push(foreshadowing_index_record(item));
    }
    tx.commit().await?;
    Ok(result)
}

fn resolve_draft_identity(
    draft: &ForeshadowingAnalysisDraft,
    chapter_number: i32,
    current: &BTreeMap<i64, foreshadowing_index::Model>,
    history: &[material_fact_change::Model],
) -> Result<Option<i64>, ForeshadowingDbError> {
    if let Some(id) = draft.existing_id {
        return Ok(Some(id));
    }
    let withdrawn_ids: BTreeSet<_> = history
        .iter()
        .filter(|record| record.status == "retracted" && record.before_json.is_null())
        .filter_map(|record| record.target_id)
        .collect();
    let candidates: BTreeSet<_> = history
        .iter()
        .filter(|record| {
            record.evidence_json.as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item.get("quote").and_then(Value::as_str) == Some(draft.evidence.as_str())
                })
            })
        })
        .filter_map(|record| record.target_id)
        .filter(|id| current.contains_key(id) || withdrawn_ids.contains(id))
        .chain(
            current
                .values()
                .filter(|item| {
                    item.setup_chapter == chapter_number && item.description == draft.description
                })
                .map(|item| item.id),
        )
        .collect();
    if candidates.len() > 1 {
        return Err(ForeshadowingDbError::InvalidArgument(
            "原文证据对应多个伏笔，请使用已有伏笔 ID 明确身份".into(),
        ));
    }
    Ok(candidates.into_iter().next())
}

fn require_evidence(
    chapter: &chapter::Model,
    evidence: &[String],
) -> Result<(), ForeshadowingDbError> {
    if evidence.is_empty()
        || evidence
            .iter()
            .any(|quote| quote.trim().is_empty() || !chapter.content.contains(quote))
    {
        return Err(ForeshadowingDbError::InvalidArgument(
            "伏笔事实必须有当前正文中的原文证据".into(),
        ));
    }
    Ok(())
}

pub(crate) fn foreshadowing_fact_snapshot(item: &foreshadowing_index::Model) -> Value {
    json!({"description":item.description,"setup_chapter":item.setup_chapter,"payoff_chapter":item.payoff_chapter,"status":item.status,"related_characters":item.related_characters,"importance":item.importance})
}

async fn record_change(
    tx: &DatabaseTransaction,
    chapter: &chapter::Model,
    item: &foreshadowing_index::Model,
    before: Value,
    evidence: Vec<String>,
    reason: &str,
) -> Result<(), ForeshadowingDbError> {
    let source =
        crate::fact_feedback::observation_source(&crate::project::chapter_record(chapter.clone()));
    crate::fact_feedback::record_fact_change(
        tx,
        crate::fact_feedback::FactChangeRecordInput {
            project_id: chapter.project_id,
            workflow_run_id: None,
            material_kind: "foreshadowing".into(),
            target_id: Some(item.id),
            field_path: String::new(),
            operation: if before.is_null() { "add" } else { "replace" }.into(),
            before,
            after: foreshadowing_fact_snapshot(item),
            sources: vec![source],
            evidence: Value::Array(
                evidence
                    .into_iter()
                    .map(|quote| json!({"chapter_number":chapter.chapter_number,"quote":quote}))
                    .collect(),
            ),
            reason: reason.into(),
            status: "applied".into(),
        },
    )
    .await
    .map_err(|error| ForeshadowingDbError::InvalidState {
        current: "foreshadowing_fact_log".into(),
        operation: error.to_string(),
    })?;
    Ok(())
}

pub(crate) async fn restore_foreshadowing_fact<C: ConnectionTrait>(
    db: &C,
    change: &material_fact_change::Model,
) -> Result<(), crate::fact_feedback::FactFeedbackDbError> {
    let Some(id) = change.target_id else {
        return Ok(());
    };
    let Some(item) = foreshadowing_index::Entity::find_by_id(id).one(db).await? else {
        return Ok(());
    };
    if item.project_id != change.project_id
        || foreshadowing_fact_snapshot(&item) != change.after_json
    {
        return Ok(());
    }
    if change.before_json.is_null() {
        foreshadowing_index::Entity::delete_by_id(id)
            .exec(db)
            .await?;
    } else {
        let before = &change.before_json;
        let mut active: foreshadowing_index::ActiveModel = item.into();
        active.description = Set(serde_json::from_value(before["description"].clone())?);
        active.setup_chapter = Set(serde_json::from_value(before["setup_chapter"].clone())?);
        active.payoff_chapter = Set(serde_json::from_value(before["payoff_chapter"].clone())?);
        active.status = Set(serde_json::from_value(before["status"].clone())?);
        active.related_characters = Set(serde_json::from_value(
            before["related_characters"].clone(),
        )?);
        active.importance = Set(serde_json::from_value(before["importance"].clone())?);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await?;
    }
    Ok(())
}
