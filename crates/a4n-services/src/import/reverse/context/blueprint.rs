use a4n_core::models::{
    chapter::{chapter, part_outline},
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, novel_blueprint, novel_project},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};

use super::chapter_entries::{
    chapter_part_entry, chapter_summary_for_reverse, sample_chapter_entry, sampled_chapters,
};
use super::extraction::{
    collect_analysis_foreshadowing, collect_extracted_characters, dedup_character_records,
};

pub(super) async fn blueprint_context(
    db: &impl ConnectionTrait,
    project: novel_project::Model,
    chapters: Vec<chapter::Model>,
    parts: Vec<part_outline::Model>,
    limit: usize,
) -> Result<Value, String> {
    let sampled = sampled_chapters(&chapters, 10);
    let sampled_summaries = sampled_chapters(&chapters, limit)
        .into_iter()
        .map(chapter_summary_for_reverse)
        .collect::<Vec<_>>();
    let sample_chapters = sampled
        .into_iter()
        .map(sample_chapter_entry)
        .collect::<Result<Vec<_>, _>>()?;
    let mut extracted_foreshadowing = collect_analysis_foreshadowing(&chapters, 30);
    let stored_foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project.id))
        .order_by_asc(foreshadowing_index::Column::SetupChapter)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    for item in stored_foreshadowing {
        if extracted_foreshadowing.len() >= 40 {
            break;
        }
        extracted_foreshadowing.push(json!({
            "chapter": item.setup_chapter,
            "content": item.description,
            "payoff_chapter": item.payoff_chapter,
            "status": item.status,
            "importance": item.importance,
            "related_characters": item.related_characters,
        }));
    }

    let existing_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project.id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .map(|blueprint| {
            json!({
                "title": blueprint.title,
                "genre": blueprint.genre,
                "style": blueprint.style,
                "tone": blueprint.tone,
                "target_audience": blueprint.target_audience,
                "one_sentence_summary": blueprint.one_sentence_summary,
                "full_synopsis": blueprint.full_synopsis,
                "world_setting": blueprint.world_setting,
            })
        });
    let existing_characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project.id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|character| {
            json!({
                "name": character.name,
                "character_type": character.character_type,
                "gender": character.gender,
                "age": character.age,
                "appearance": character.appearance,
                "personality": character.personality,
                "background": character.background,
                "motivation": character.motivation,
                "arc_summary": character.arc_summary,
                "relationship_to_protagonist": character.relationship_to_protagonist,
                "arc_stages": character.arc_stages,
            })
        })
        .collect::<Vec<_>>();
    let protagonists = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project.id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|profile| {
            json!({
                "name": profile.name,
                "gender": profile.gender,
                "age": profile.age,
                "base_attributes": profile.base_attributes,
                "current_state": profile.current_state,
                "character_arc": profile.character_arc,
                "appearance": profile.appearance,
                "background": profile.background,
                "personality": profile.personality,
            })
        })
        .collect::<Vec<_>>();
    let mut extracted_characters = collect_extracted_characters(&chapters, 40);
    extracted_characters.extend(existing_characters);
    extracted_characters.extend(protagonists);
    let extracted_characters = dedup_character_records(extracted_characters);

    Ok(json!({
        "novel_title": project.title,
        "total_chapters": chapters.len(),
        "sampled_chapters_count": sampled_summaries.len(),
        "chapter_summaries": sampled_summaries,
        "part_outlines": parts.iter().map(chapter_part_entry).collect::<Vec<_>>(),
        "sample_chapters": sample_chapters,
        "extracted_characters": extracted_characters.into_iter().take(40).collect::<Vec<_>>(),
        "extracted_foreshadowing": extracted_foreshadowing,
        "existing_blueprint": existing_blueprint,
    }))
}
