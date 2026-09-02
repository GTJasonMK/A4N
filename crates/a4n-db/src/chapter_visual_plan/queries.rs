use a4n_core::models::chapter::chapter_visual_plan;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
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
        .filter(chapter_visual_plan::Column::RootProjectId.eq(source.root_project_id))
        .filter(chapter_visual_plan::Column::SourceChapterId.eq(source.source_chapter_id))
        .filter(chapter_visual_plan::Column::SourceRevisionKey.eq(&source.source_revision_key))
        .filter(chapter_visual_plan::Column::SourceContentSha256.eq(&source.content_sha256))
        .filter(chapter_visual_plan::Column::Status.is_in(["draft", "confirmed"]))
        .order_by_desc(chapter_visual_plan::Column::UpdatedAt)
        .order_by_desc(chapter_visual_plan::Column::Id)
        .all(db)
        .await?;
    let mut result = ChapterVisualPlanSet::default();
    for row in rows {
        let parsed = parse_record(row)?;
        match parsed.record.status.as_str() {
            "draft" if result.draft.is_none() => result.draft = Some(parsed),
            "confirmed" if result.confirmed.is_none() => result.confirmed = Some(parsed),
            _ => {}
        }
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

    let mut rows = chapter_visual_plan::Entity::find()
        .filter(source_condition(sources))
        .filter(chapter_visual_plan::Column::Status.is_in(["draft", "confirmed"]))
        .distinct_on([
            chapter_visual_plan::Column::RootProjectId,
            chapter_visual_plan::Column::SourceChapterId,
            chapter_visual_plan::Column::SourceRevisionKey,
            chapter_visual_plan::Column::SourceContentSha256,
            chapter_visual_plan::Column::Status,
        ])
        .order_by_asc(chapter_visual_plan::Column::RootProjectId)
        .order_by_asc(chapter_visual_plan::Column::SourceChapterId)
        .order_by_asc(chapter_visual_plan::Column::SourceRevisionKey)
        .order_by_asc(chapter_visual_plan::Column::SourceContentSha256)
        .order_by_asc(chapter_visual_plan::Column::Status)
        .order_by_desc(chapter_visual_plan::Column::UpdatedAt)
        .order_by_desc(chapter_visual_plan::Column::Id)
        .all(db)
        .await?;
    rows.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.id.cmp(&left.id))
    });
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
                match parsed.record.status.as_str() {
                    "draft" if plans.draft.is_none() => plans.draft = Some(parsed),
                    "confirmed" if plans.confirmed.is_none() => plans.confirmed = Some(parsed),
                    _ => {}
                }
            }
        }
        plans_by_source.insert(source_key, plans);
    }

    Ok(sources
        .iter()
        .map(|source| {
            plans_by_source
                .get(&source_identity(source))
                .cloned()
                .unwrap_or_default()
        })
        .collect())
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
    super::codec::validate_sha256(&source.content_sha256, "source_content_sha256")
}

type SourceIdentity = (i64, i64, String, String);

fn source_identity(source: &ResolvedChapterSource) -> SourceIdentity {
    (
        source.root_project_id,
        source.source_chapter_id,
        source.source_revision_key.clone(),
        source.content_sha256.clone(),
    )
}

fn source_identity_from_record(
    record: &a4n_core::models::chapter::chapter_visual_plan::Model,
) -> SourceIdentity {
    (
        record.root_project_id,
        record.source_chapter_id,
        record.source_revision_key.clone(),
        record.source_content_sha256.clone(),
    )
}

fn source_condition(sources: &[ResolvedChapterSource]) -> Condition {
    sources.iter().fold(Condition::any(), |condition, source| {
        condition.add(
            Condition::all()
                .add(chapter_visual_plan::Column::RootProjectId.eq(source.root_project_id))
                .add(chapter_visual_plan::Column::SourceChapterId.eq(source.source_chapter_id))
                .add(chapter_visual_plan::Column::SourceRevisionKey.eq(&source.source_revision_key))
                .add(chapter_visual_plan::Column::SourceContentSha256.eq(&source.content_sha256)),
        )
    })
}
