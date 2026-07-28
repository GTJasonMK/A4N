use a4n_core::models::chapter::chapter_shooting_script;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

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
        .filter(chapter_shooting_script::Column::SourceContentSha256.eq(&source.content_sha256))
        .order_by_desc(chapter_shooting_script::Column::UpdatedAt)
        .order_by_desc(chapter_shooting_script::Column::Id)
        .one(db)
        .await?
        .map(parse_record)
        .transpose()
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
