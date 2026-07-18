use a4n_core::models::chapter::part_outline;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use super::super::outline_records::ProjectOutlineRecordError;
use super::contracts::ImportPartOutlineUpsertInput;

pub async fn upsert_import_part_outline(
    db: &DatabaseConnection,
    input: ImportPartOutlineUpsertInput,
) -> Result<part_outline::Model, ProjectOutlineRecordError> {
    let now = Utc::now().naive_utc();
    let existing = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(input.project_id))
        .filter(part_outline::Column::PartNumber.eq(input.part_number))
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;

    if let Some(existing) = existing {
        let mut active: part_outline::ActiveModel = existing.into();
        active.title = Set(input.title);
        active.start_chapter = Set(input.start_chapter);
        active.end_chapter = Set(input.end_chapter);
        active.summary = Set(Some(input.summary));
        active.theme = Set(Some(input.theme));
        active.key_events = Set(Some(input.key_events));
        active.character_focus = Set(Some(input.character_focus));
        active.updated_at = Set(now);
        active
            .update(db)
            .await
            .map_err(ProjectOutlineRecordError::from)
    } else {
        part_outline::ActiveModel {
            project_id: Set(input.project_id),
            part_number: Set(input.part_number),
            title: Set(input.title),
            start_chapter: Set(input.start_chapter),
            end_chapter: Set(input.end_chapter),
            summary: Set(Some(input.summary)),
            theme: Set(Some(input.theme)),
            key_events: Set(Some(input.key_events)),
            character_focus: Set(Some(input.character_focus)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectOutlineRecordError::from)
    }
}
