use a4n_core::models::chapter::chapter_shooting_script;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::{
    codec::parse_record,
    contracts::{ParsedChapterShootingScript, ShootingScriptDbError},
    validation::{require_positive, require_text},
};

pub async fn chapter_shooting_script_for_source<C>(
    db: &C,
    root_project_id: i64,
    source_chapter_id: i64,
    source_revision_key: &str,
) -> Result<Option<ParsedChapterShootingScript>, ShootingScriptDbError>
where
    C: ConnectionTrait,
{
    require_positive(root_project_id, "root_project_id")?;
    require_positive(source_chapter_id, "source_chapter_id")?;
    let source_revision_key = require_text(source_revision_key, "source_revision_key")?;
    chapter_shooting_script::Entity::find()
        .filter(chapter_shooting_script::Column::RootProjectId.eq(root_project_id))
        .filter(chapter_shooting_script::Column::SourceChapterId.eq(source_chapter_id))
        .filter(chapter_shooting_script::Column::SourceRevisionKey.eq(source_revision_key))
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
