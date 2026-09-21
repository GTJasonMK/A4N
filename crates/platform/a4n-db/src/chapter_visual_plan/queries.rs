use a4n_db_models::models::chapter::chapter_visual_plan;
use a4n_domain::chapter_visual_plan::ChapterVisualPlanStatus;
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;

use super::{
    codec::parse_record,
    contracts::{ChapterVisualPlanDbError, ChapterVisualPlanSet, ParsedChapterVisualPlan},
};
use crate::project::ResolvedChapterSource;

pub async fn chapter_visual_plans_for_source<C>(
    db: &C,
    source: &ResolvedChapterSource,
) -> Result<ChapterVisualPlanSet, ChapterVisualPlanDbError>
where
    C: ConnectionTrait,
{
    validate_source(source)?;
    let rows = chapter_visual_plan::Entity::find()
        .filter(lineage_condition(source)?)
        .filter(chapter_visual_plan::Column::Status.is_in(["draft", "confirmed"]))
        .all(db)
        .await?;
    let mut result = ChapterVisualPlanSet::default();
    for row in rows {
        let parsed = parse_record(row)?;
        insert_live_plan(&mut result, parsed)?;
    }
    Ok(result)
}

/// 在同一批来源内读取章节视觉方案，按输入顺序返回结果。
pub async fn chapter_visual_plans_for_sources<C>(
    db: &C,
    sources: &[ResolvedChapterSource],
) -> Result<Vec<ChapterVisualPlanSet>, ChapterVisualPlanDbError>
where
    C: ConnectionTrait,
{
    for source in sources {
        validate_source(source)?;
    }
    if sources.is_empty() {
        return Ok(Vec::new());
    }

    let rows = chapter_visual_plan::Entity::find()
        .filter(source_condition(sources)?)
        .filter(chapter_visual_plan::Column::Status.is_in(["draft", "confirmed"]))
        .all(db)
        .await?;
    let mut rows_by_source = HashMap::<SourceIdentity, Vec<chapter_visual_plan::Model>>::new();
    for row in rows {
        rows_by_source
            .entry(source_identity_from_record(&row))
            .or_default()
            .push(row);
    }

    let mut plans_by_source = HashMap::<SourceIdentity, ChapterVisualPlanSet>::new();
    for source in sources {
        let source_key = source_identity(source);
        if plans_by_source.contains_key(&source_key) {
            continue;
        }
        let mut plans = ChapterVisualPlanSet::default();
        if let Some(rows) = rows_by_source.remove(&source_key) {
            for row in rows {
                let parsed = parse_record(row)?;
                insert_live_plan(&mut plans, parsed)?;
            }
        }
        plans_by_source.insert(source_key, plans);
    }

    sources
        .iter()
        .map(|source| {
            plans_by_source
                .get(&source_identity(source))
                .cloned()
                .ok_or_else(|| {
                    ChapterVisualPlanDbError::InvalidState(format!(
                        "章节视觉编排来源索引丢失: root_project_id={}, source_chapter_id={}",
                        source.root_project_id, source.source_chapter_id
                    ))
                })
        })
        .collect()
}

pub async fn confirmed_chapter_visual_plan_for_source<C>(
    db: &C,
    source: &ResolvedChapterSource,
) -> Result<Option<ParsedChapterVisualPlan>, ChapterVisualPlanDbError>
where
    C: ConnectionTrait,
{
    Ok(chapter_visual_plans_for_source(db, source).await?.confirmed)
}

pub async fn chapter_visual_plan_by_id<C>(
    db: &C,
    record_id: i64,
) -> Result<ParsedChapterVisualPlan, ChapterVisualPlanDbError>
where
    C: ConnectionTrait,
{
    if record_id <= 0 {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "record_id 必须大于 0".into(),
        ));
    }
    let row = chapter_visual_plan::Entity::find_by_id(record_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            ChapterVisualPlanDbError::NotFound(format!("章节视觉编排不存在: {record_id}"))
        })?;
    parse_record(row)
}

pub(super) fn validate_source(
    source: &ResolvedChapterSource,
) -> Result<(), ChapterVisualPlanDbError> {
    if source.root_project_id <= 0 || source.source_chapter_id <= 0 || source.chapter_number <= 0 {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "章节视觉编排来源 ID 和章节号必须大于 0".into(),
        ));
    }
    if !super::codec::valid_source_snapshot_identity(
        source.source_snapshot_id,
        source.source_snapshot_chapter_id,
    ) {
        return Err(ChapterVisualPlanDbError::InvalidArgument(
            "source_snapshot_id 与 source_snapshot_chapter_id 必须同时为空或同时为正数".into(),
        ));
    }
    super::codec::validate_sha256(&source.content_sha256, "source_content_sha256")
}

type SourceIdentity = (i64, i64, Option<i64>, Option<i64>);

fn source_identity(source: &ResolvedChapterSource) -> SourceIdentity {
    (
        source.root_project_id,
        source.source_chapter_id,
        source.source_snapshot_id,
        source.source_snapshot_chapter_id,
    )
}

fn source_identity_from_record(
    record: &a4n_db_models::models::chapter::chapter_visual_plan::Model,
) -> SourceIdentity {
    (
        record.root_project_id,
        record.source_chapter_id,
        record.source_snapshot_id,
        record.source_snapshot_chapter_id,
    )
}

fn insert_live_plan(
    plans: &mut ChapterVisualPlanSet,
    parsed: ParsedChapterVisualPlan,
) -> Result<(), ChapterVisualPlanDbError> {
    let (slot, status) = match parsed.status {
        ChapterVisualPlanStatus::Draft => (&mut plans.draft, "draft"),
        ChapterVisualPlanStatus::Confirmed => (&mut plans.confirmed, "confirmed"),
        status => {
            return Err(ChapterVisualPlanDbError::InvalidState(format!(
                "查询到不受支持的章节视觉编排状态: {}",
                status.as_str()
            )));
        }
    };
    if slot.is_some() {
        return Err(ChapterVisualPlanDbError::InvalidState(format!(
            "章节视觉编排存在重复的 {status} lineage: record_id={}",
            parsed.record.id
        )));
    }
    *slot = Some(parsed);
    Ok(())
}

fn source_condition(
    sources: &[ResolvedChapterSource],
) -> Result<Condition, ChapterVisualPlanDbError> {
    sources
        .iter()
        .try_fold(Condition::any(), |condition, source| {
            Ok(condition.add(lineage_condition(source)?))
        })
}

fn lineage_condition(
    source: &ResolvedChapterSource,
) -> Result<Condition, ChapterVisualPlanDbError> {
    lineage_condition_for_identity(
        source.root_project_id,
        source.source_chapter_id,
        source.source_snapshot_id,
        source.source_snapshot_chapter_id,
    )
}

pub(super) fn lineage_condition_for_identity(
    root_project_id: i64,
    source_chapter_id: i64,
    source_snapshot_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
) -> Result<Condition, ChapterVisualPlanDbError> {
    let mut condition = Condition::all()
        .add(chapter_visual_plan::Column::RootProjectId.eq(root_project_id))
        .add(chapter_visual_plan::Column::SourceChapterId.eq(source_chapter_id));
    match (source_snapshot_id, source_snapshot_chapter_id) {
        (Some(snapshot_id), Some(snapshot_chapter_id)) => {
            condition = condition
                .add(chapter_visual_plan::Column::SourceSnapshotId.eq(snapshot_id))
                .add(chapter_visual_plan::Column::SourceSnapshotChapterId.eq(snapshot_chapter_id));
        }
        (None, None) => {
            condition = condition
                .add(chapter_visual_plan::Column::SourceSnapshotId.is_null())
                .add(chapter_visual_plan::Column::SourceSnapshotChapterId.is_null());
        }
        _ => {
            return Err(ChapterVisualPlanDbError::InvalidState(
                "章节视觉编排 source snapshot lineage 身份不完整".into(),
            ));
        }
    }
    Ok(condition)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_snapshot_identity_is_rejected() {
        let error = lineage_condition_for_identity(1, 2, Some(3), None)
            .expect_err("incomplete snapshot identity");
        assert!(matches!(error, ChapterVisualPlanDbError::InvalidState(_)));
    }
}
