use super::chapter_outline::{
    analysis_metadata_array, analysis_value_path, chapter_outline_summary,
    collect_part_array_values,
};
use a4n_core::models::chapter::{chapter, part_outline};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;

pub(super) async fn upsert_import_part_outlines(
    db: &impl ConnectionTrait,
    project_id: i64,
    chapters: &[chapter::Model],
) -> Result<usize, String> {
    const CHAPTERS_PER_PART: i32 = 25;
    if chapters.len() < 50 {
        return Ok(0);
    }

    let mut generated = 0usize;
    let mut start = 1i32;
    let max_chapter = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .ok_or_else(|| "项目没有可用于生成分部大纲的章节".to_string())?;

    while start <= max_chapter {
        let end = (start + CHAPTERS_PER_PART - 1).min(max_chapter);
        let part_number = ((start - 1) / CHAPTERS_PER_PART) + 1;
        let part_chapters = chapters
            .iter()
            .filter(|chapter| chapter.chapter_number >= start && chapter.chapter_number <= end)
            .collect::<Vec<_>>();
        if part_chapters.is_empty() {
            start = end + 1;
            continue;
        }

        let summary = part_chapters
            .iter()
            .map(|chapter| {
                Ok(format!(
                    "第{}章：{}",
                    chapter.chapter_number,
                    chapter_outline_summary(chapter)?
                ))
            })
            .collect::<Result<Vec<_>, String>>()?
            .join("\n");
        let key_events = Value::Array(collect_part_array_values(&part_chapters, 30, |chapter| {
            analysis_value_path(
                chapter.analysis_data.as_ref(),
                &["key_events"],
                chapter.chapter_number,
            )
        })?);
        let characters = Value::Array(collect_part_array_values(&part_chapters, 30, |chapter| {
            analysis_metadata_array(
                chapter.analysis_data.as_ref(),
                "characters",
                chapter.chapter_number,
            )
        })?);

        let existing = part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .filter(part_outline::Column::PartNumber.eq(part_number))
            .one(db)
            .await
            .map_err(|e| e.to_string())?;
        let now = Utc::now().naive_utc();
        let title = format!("第{part_number}部分");

        if let Some(existing) = existing {
            let mut active: part_outline::ActiveModel = existing.into();
            active.title = Set(title);
            active.start_chapter = Set(start);
            active.end_chapter = Set(end);
            active.summary = Set(Some(summary));
            active.theme = Set(Some("导入分析".to_string()));
            active.key_events = Set(Some(key_events));
            active.character_focus = Set(Some(characters));
            active.updated_at = Set(now);
            active.update(db).await.map_err(|e| e.to_string())?;
        } else {
            part_outline::ActiveModel {
                project_id: Set(project_id),
                part_number: Set(part_number),
                title: Set(title),
                start_chapter: Set(start),
                end_chapter: Set(end),
                summary: Set(Some(summary)),
                theme: Set(Some("导入分析".to_string())),
                key_events: Set(Some(key_events)),
                character_focus: Set(Some(characters)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|e| e.to_string())?;
        }

        generated += 1;
        start = end + 1;
    }
    Ok(generated)
}
