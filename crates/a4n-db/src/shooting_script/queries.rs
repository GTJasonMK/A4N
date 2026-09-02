use a4n_core::models::chapter::chapter_shooting_script;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::HashMap;

use super::{
    codec::parse_record,
    contracts::{ParsedChapterShootingScript, ShootingScriptDbError},
    validation::{require_positive, validate_source},
};
use crate::project::ResolvedChapterSource;

pub async fn chapter_shooting_script_for_source<C>(
    db: &C,
    source: &ResolvedChapterSource,
) -> Result<Option<ParsedChapterShootingScript>, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    validate_source(source)?;
    chapter_shooting_script::Entity::find()
        .filter(chapter_shooting_script::Column::RootProjectId.eq(source.root_project_id))
        .filter(chapter_shooting_script::Column::SourceChapterId.eq(source.source_chapter_id))
        .filter(chapter_shooting_script::Column::SourceRevisionKey.eq(&source.source_revision_key))
        .filter(chapter_shooting_script::Column::SourceContentSha256.eq(&source.content_sha256))
        .order_by_desc(chapter_shooting_script::Column::UpdatedAt)
        .order_by_desc(chapter_shooting_script::Column::Id)
        .one(db)
        .await?
        .map(parse_record)
        .transpose()
}

/// 在同一批来源内读取最新章节拍摄剧本，按输入顺序返回结果。
pub async fn chapter_shooting_scripts_for_sources<C>(
    db: &C,
    sources: &[ResolvedChapterSource],
) -> Result<Vec<Option<ParsedChapterShootingScript>>, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    for source in sources {
        validate_source(source)?;
    }
    if sources.is_empty() {
        return Ok(Vec::new());
    }

    let rows = chapter_shooting_script::Entity::find()
        .filter(source_condition(sources))
        .distinct_on([
            chapter_shooting_script::Column::RootProjectId,
            chapter_shooting_script::Column::SourceChapterId,
            chapter_shooting_script::Column::SourceRevisionKey,
            chapter_shooting_script::Column::SourceContentSha256,
        ])
        .order_by_asc(chapter_shooting_script::Column::RootProjectId)
        .order_by_asc(chapter_shooting_script::Column::SourceChapterId)
        .order_by_asc(chapter_shooting_script::Column::SourceRevisionKey)
        .order_by_asc(chapter_shooting_script::Column::SourceContentSha256)
        .order_by_desc(chapter_shooting_script::Column::UpdatedAt)
        .order_by_desc(chapter_shooting_script::Column::Id)
        .all(db)
        .await?;
    let mut rows_by_source = HashMap::<SourceIdentity, chapter_shooting_script::Model>::new();
    for row in rows {
        let source_key = source_identity_from_record(&row);
        rows_by_source.entry(source_key).or_insert(row);
    }

    let mut scripts_by_source =
        HashMap::<SourceIdentity, Option<ParsedChapterShootingScript>>::new();
    for source in sources {
        let source_key = source_identity(source);
        if scripts_by_source.contains_key(&source_key) {
            continue;
        }
        let script = rows_by_source
            .remove(&source_key)
            .map(parse_record)
            .transpose()?;
        scripts_by_source.insert(source_key, script);
    }

    Ok(sources
        .iter()
        .map(|source| {
            scripts_by_source
                .get(&source_identity(source))
                .cloned()
                .unwrap_or_default()
        })
        .collect())
}

pub async fn chapter_shooting_script_by_id<C>(
    db: &C,
    record_id: i64,
) -> Result<ParsedChapterShootingScript, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    require_positive(record_id, "record_id")?;
    let record = chapter_shooting_script::Entity::find_by_id(record_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            ShootingScriptDbError::NotFound(format!("章节拍摄剧本不存在: {record_id}"))
        })?;
    parse_record(record)
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

fn source_identity_from_record(record: &chapter_shooting_script::Model) -> SourceIdentity {
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
                .add(chapter_shooting_script::Column::RootProjectId.eq(source.root_project_id))
                .add(chapter_shooting_script::Column::SourceChapterId.eq(source.source_chapter_id))
                .add(
                    chapter_shooting_script::Column::SourceRevisionKey
                        .eq(&source.source_revision_key),
                )
                .add(
                    chapter_shooting_script::Column::SourceContentSha256.eq(&source.content_sha256),
                ),
        )
    })
}
