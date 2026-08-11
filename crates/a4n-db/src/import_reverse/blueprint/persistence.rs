use a4n_core::models::project::novel_blueprint;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::import_reverse::{ImportReverseDbError, ReverseBlueprintApplyRecord};

pub async fn apply_reverse_blueprint_record<C>(
    db: &C,
    project_id: i64,
    record: &ReverseBlueprintApplyRecord,
) -> Result<novel_blueprint::Model, ImportReverseDbError>
where
    C: ConnectionTrait,
{
    let now = record.applied_at;
    let existing_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await?;

    if let Some(existing) = existing_blueprint {
        let mut active: novel_blueprint::ActiveModel = existing.into();
        active.title = Set(record.title.clone());
        active.genre = Set(record.genre.clone());
        active.style = Set(Some(record.style.clone()));
        active.tone = Set(Some(record.tone.clone()));
        active.target_audience = Set(Some(record.target_audience.clone()));
        active.one_sentence_summary = Set(Some(record.one_sentence_summary.clone()));
        active.full_synopsis = Set(Some(record.full_synopsis.clone()));
        active.world_setting = Set(Some(record.world_setting.clone()));
        if let Some(total_chapters) = record.total_chapters {
            active.total_chapters = Set(Some(total_chapters));
        }
        active.needs_part_outlines = Set(record.needs_part_outlines);
        active.chapters_per_part = Set(record.chapters_per_part);
        active.current_version = Set(record.current_version.clone());
        active.updated_at = Set(now);
        Ok(active.update(db).await?)
    } else {
        Ok(novel_blueprint::ActiveModel {
            project_id: Set(project_id),
            title: Set(record.title.clone()),
            genre: Set(record.genre.clone()),
            style: Set(Some(record.style.clone())),
            tone: Set(Some(record.tone.clone())),
            target_audience: Set(Some(record.target_audience.clone())),
            one_sentence_summary: Set(Some(record.one_sentence_summary.clone())),
            full_synopsis: Set(Some(record.full_synopsis.clone())),
            world_setting: Set(Some(record.world_setting.clone())),
            current_version: Set(record.current_version.clone()),
            needs_part_outlines: Set(record.needs_part_outlines),
            total_chapters: Set(record.total_chapters),
            chapters_per_part: Set(record.chapters_per_part),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?)
    }
}
