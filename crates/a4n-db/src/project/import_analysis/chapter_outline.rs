use a4n_core::models::chapter::chapter_outline;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use super::super::outline_records::ProjectOutlineRecordError;
use super::contracts::ImportChapterOutlineUpsertInput;

pub async fn upsert_import_chapter_outline(
    db: &DatabaseConnection,
    input: ImportChapterOutlineUpsertInput,
) -> Result<chapter_outline::Model, ProjectOutlineRecordError> {
    let now = Utc::now().naive_utc();
    let existing = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(input.chapter_number))
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;

    if let Some(existing) = existing {
        let mut active: chapter_outline::ActiveModel = existing.into();
        active.title = Set(Some(input.title));
        active.summary = Set(Some(input.summary));
        active.key_scenes = Set(input.key_scenes);
        active.foreshadowing_setup = Set(input.foreshadowing_setup);
        active.foreshadowing_payoff = Set(input.foreshadowing_payoff);
        active.characters_involved = Set(input.characters_involved);
        active.status = Set("completed".to_string());
        active.updated_at = Set(now);
        active
            .update(db)
            .await
            .map_err(ProjectOutlineRecordError::from)
    } else {
        chapter_outline::ActiveModel {
            project_id: Set(input.project_id),
            part_id: Set(None),
            chapter_number: Set(input.chapter_number),
            title: Set(Some(input.title)),
            summary: Set(Some(input.summary)),
            beats: Set(None),
            key_scenes: Set(input.key_scenes),
            pacing_score: Set(None),
            foreshadowing_setup: Set(input.foreshadowing_setup),
            foreshadowing_payoff: Set(input.foreshadowing_payoff),
            characters_involved: Set(input.characters_involved),
            word_count_target: Set(Some(input.word_count_target)),
            status: Set("completed".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectOutlineRecordError::from)
    }
}
