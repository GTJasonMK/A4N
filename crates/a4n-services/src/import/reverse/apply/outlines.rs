use super::validation::{
    optional_positive_i32, required_array_clone, required_object_clone, required_positive_i32,
    validate_reverse_outline_items, validate_reverse_part_outline_items,
};
use crate::import::reverse_json::{reject_unknown_keys, required_array, required_string};
use a4n_core::models::chapter::{chapter_outline, part_outline};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};

pub async fn apply_reverse_outline_data(
    db: &impl ConnectionTrait,
    project_id: i64,
    data: &Value,
) -> Result<Value, String> {
    reject_unknown_keys(data, &["outlines"], "reverse_outline")?;
    let outlines = required_array(data, "outlines", "reverse_outline")?;
    validate_reverse_outline_items(outlines)?;
    let now = Utc::now().naive_utc();
    let mut outline_count = 0usize;

    for (index, item) in outlines.iter().enumerate() {
        let context = format!("reverse_outline.outlines[{index}]");
        let chapter_number = required_positive_i32(item, "chapter_number", &context)?;
        let title = required_string(item, "title", &context)?;
        let summary = required_string(item, "summary", &context)?;
        let existing = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(existing) = existing {
            let mut active: chapter_outline::ActiveModel = existing.into();
            active.title = Set(Some(title));
            active.summary = Set(Some(summary));
            if let Some(word_count_target) =
                optional_positive_i32(item, "word_count_target", &context)?
            {
                active.word_count_target = Set(Some(word_count_target));
            }
            active.status = Set("completed".to_string());
            active.updated_at = Set(now);
            active.update(db).await.map_err(|e| e.to_string())?;
        } else {
            chapter_outline::ActiveModel {
                project_id: Set(project_id),
                part_id: Set(None),
                chapter_number: Set(chapter_number),
                title: Set(Some(title)),
                summary: Set(Some(summary)),
                beats: Set(None),
                key_scenes: Set(None),
                pacing_score: Set(None),
                foreshadowing_setup: Set(None),
                foreshadowing_payoff: Set(None),
                characters_involved: Set(None),
                word_count_target: Set(optional_positive_i32(item, "word_count_target", &context)?),
                status: Set("completed".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
        }
        outline_count += 1;
    }

    Ok(json!({ "outline_count": outline_count }))
}

pub async fn apply_reverse_part_outline_data(
    db: &impl ConnectionTrait,
    project_id: i64,
    data: &Value,
) -> Result<Value, String> {
    reject_unknown_keys(data, &["parts"], "reverse_part_outline")?;
    let parts = required_array(data, "parts", "reverse_part_outline")?;
    validate_reverse_part_outline_items(parts)?;
    let now = Utc::now().naive_utc();
    let mut part_count = 0usize;

    for (index, item) in parts.iter().enumerate() {
        let context = format!("reverse_part_outline.parts[{index}]");
        let part_number = required_positive_i32(item, "part_number", &context)?;
        let title = required_string(item, "title", &context)?;
        let start_chapter = required_positive_i32(item, "start_chapter", &context)?;
        let end_chapter = required_positive_i32(item, "end_chapter", &context)?;
        let summary = required_string(item, "summary", &context)?;
        let theme = required_string(item, "theme", &context)?;
        let key_events = required_array_clone(item, "key_events", &context)?;
        let character_focus = required_object_clone(item, "character_focus", &context)?;
        let existing = part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .filter(part_outline::Column::PartNumber.eq(part_number))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(existing) = existing {
            let mut active: part_outline::ActiveModel = existing.into();
            active.title = Set(title);
            active.start_chapter = Set(start_chapter);
            active.end_chapter = Set(end_chapter);
            active.summary = Set(Some(summary));
            active.theme = Set(Some(theme));
            active.key_events = Set(Some(key_events));
            active.character_focus = Set(Some(character_focus));
            active.updated_at = Set(now);
            active.update(db).await.map_err(|e| e.to_string())?;
        } else {
            part_outline::ActiveModel {
                project_id: Set(project_id),
                part_number: Set(part_number),
                title: Set(title),
                start_chapter: Set(start_chapter),
                end_chapter: Set(end_chapter),
                summary: Set(Some(summary)),
                theme: Set(Some(theme)),
                key_events: Set(Some(key_events)),
                character_focus: Set(Some(character_focus)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
        }
        part_count += 1;
    }

    Ok(json!({ "part_outline_count": part_count }))
}
