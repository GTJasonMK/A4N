use super::import_payload::{
    ImportBlueprint, ImportBlueprintVersion, ImportChapterOutline, ImportCharacter,
    ImportPartOutline, ImportRelationship,
};
use a4n_core::models::{
    chapter::{chapter_outline, part_outline},
    project::{blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint},
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use std::collections::HashMap;

pub(super) async fn insert_import_blueprint<C>(
    db: &C,
    project_id: i64,
    blueprint: &ImportBlueprint,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set(blueprint.title.clone()),
        genre: Set(blueprint.genre.clone()),
        style: Set(blueprint.style.clone()),
        tone: Set(blueprint.tone.clone()),
        target_audience: Set(blueprint.target_audience.clone()),
        one_sentence_summary: Set(blueprint.one_sentence_summary.clone()),
        full_synopsis: Set(blueprint.full_synopsis.clone()),
        world_setting: Set(blueprint.world_setting.clone()),
        current_version: Set(blueprint.current_version.clone()),
        needs_part_outlines: Set(blueprint.needs_part_outlines),
        total_chapters: Set(blueprint.total_chapters),
        chapters_per_part: Set(blueprint.chapters_per_part),
        avatar_svg: Set(blueprint.avatar_svg.clone()),
        avatar_animal: Set(blueprint.avatar_animal.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| format!("导入蓝图失败: {e}"))?;
    Ok(())
}

pub(super) async fn insert_import_blueprint_versions<C>(
    db: &C,
    project_id: i64,
    versions: &[ImportBlueprintVersion],
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for version in versions {
        blueprint_version::ActiveModel {
            project_id: Set(project_id),
            version: Set(version.version.clone()),
            change_description: Set(version.change_description.clone()),
            blueprint_data: Set(version.blueprint_data.clone()),
            world_setting_snapshot: Set(version.world_setting_snapshot.clone()),
            characters_snapshot: Set(version.characters_snapshot.clone()),
            relationships_snapshot: Set(version.relationships_snapshot.clone()),
            synopsis_snapshot: Set(version.synopsis_snapshot.clone()),
            created_at: Set(now),
            created_by: Set(version.created_by.clone()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| format!("导入蓝图版本失败: {e}"))?;
    }
    Ok(())
}

pub(super) async fn insert_import_characters<C>(
    db: &C,
    project_id: i64,
    characters: &[ImportCharacter],
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for character in characters {
        blueprint_character::ActiveModel {
            project_id: Set(project_id),
            name: Set(character.name.clone()),
            character_type: Set(character.character_type.clone()),
            gender: Set(character.gender.clone()),
            age: Set(character.age.clone()),
            appearance: Set(character.appearance.clone()),
            personality: Set(character.personality.clone()),
            background: Set(character.background.clone()),
            motivation: Set(character.motivation.clone()),
            arc_summary: Set(character.arc_summary.clone()),
            relationship_to_protagonist: Set(character.relationship_to_protagonist.clone()),
            arc_stages: Set(character.arc_stages.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| format!("导入角色失败: {e}"))?;
    }
    Ok(())
}

pub(super) async fn insert_import_relationships<C>(
    db: &C,
    project_id: i64,
    relationships: &[ImportRelationship],
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for relationship in relationships {
        blueprint_relationship::ActiveModel {
            project_id: Set(project_id),
            relationship_type: Set(relationship.relationship_type.clone()),
            character_a: Set(relationship.character_a.clone()),
            character_b: Set(relationship.character_b.clone()),
            description: Set(relationship.description.clone()),
            strength: Set(relationship.strength),
            evolution: Set(relationship.evolution.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| format!("导入角色关系失败: {e}"))?;
    }
    Ok(())
}

pub(super) async fn insert_import_part_outlines<C>(
    db: &C,
    project_id: i64,
    parts: &[ImportPartOutline],
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let mut id_map = HashMap::new();
    for part in parts {
        let inserted = part_outline::ActiveModel {
            project_id: Set(project_id),
            part_number: Set(part.part_number),
            title: Set(part.title.clone()),
            start_chapter: Set(part.start_chapter),
            end_chapter: Set(part.end_chapter),
            summary: Set(part.summary.clone()),
            theme: Set(part.theme.clone()),
            key_events: Set(part.key_events.clone()),
            character_focus: Set(part.character_focus.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| format!("导入分部大纲失败: {e}"))?;
        if let Some(old_id) = part.old_id {
            id_map.insert(old_id, inserted.id);
        }
    }
    Ok(id_map)
}

pub(super) async fn insert_import_chapter_outlines<C>(
    db: &C,
    project_id: i64,
    outlines: &[ImportChapterOutline],
    part_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let mut id_map = HashMap::new();
    for outline in outlines {
        let part_id = outline
            .old_part_id
            .map(|old_id| {
                part_id_map.get(&old_id).copied().ok_or_else(|| {
                    format!(
                        "第{}章大纲引用的分部大纲不存在: part_id={old_id}",
                        outline.chapter_number
                    )
                })
            })
            .transpose()?;
        let inserted = chapter_outline::ActiveModel {
            project_id: Set(project_id),
            part_id: Set(part_id),
            chapter_number: Set(outline.chapter_number),
            title: Set(outline.title.clone()),
            summary: Set(outline.summary.clone()),
            beats: Set(outline.beats.clone()),
            key_scenes: Set(outline.key_scenes.clone()),
            pacing_score: Set(outline.pacing_score),
            foreshadowing_setup: Set(outline.foreshadowing_setup.clone()),
            foreshadowing_payoff: Set(outline.foreshadowing_payoff.clone()),
            characters_involved: Set(outline.characters_involved.clone()),
            word_count_target: Set(outline.word_count_target),
            status: Set(outline.status.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| format!("导入章节大纲失败: {e}"))?;
        if let Some(old_id) = outline.old_id {
            id_map.insert(old_id, inserted.id);
        }
    }
    Ok(id_map)
}
