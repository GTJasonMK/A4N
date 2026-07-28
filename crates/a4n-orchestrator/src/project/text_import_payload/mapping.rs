use super::contracts::ImportPayload;
use crate::project::{
    ImportTextProjectBlueprint as DbImportBlueprint,
    ImportTextProjectBlueprintVersion as DbImportBlueprintVersion,
    ImportTextProjectChapter as DbImportChapter,
    ImportTextProjectChapterOutline as DbImportChapterOutline,
    ImportTextProjectCharacter as DbImportCharacter,
    ImportTextProjectPartOutline as DbImportPartOutline,
    ImportTextProjectPayload as DbImportPayload,
    ImportTextProjectRelationship as DbImportRelationship,
};

impl ImportPayload {
    pub(crate) fn into_db_payload(self) -> DbImportPayload {
        DbImportPayload {
            chapters: self
                .chapters
                .into_iter()
                .map(|chapter| DbImportChapter {
                    old_outline_id: chapter.old_outline_id,
                    chapter_number: chapter.chapter_number,
                    title: chapter.title,
                    content: chapter.content,
                    word_count: chapter.word_count,
                    status: chapter.status,
                    summary: chapter.summary,
                    style_tags: chapter.style_tags,
                    analysis_data: chapter.analysis_data,
                })
                .collect(),
            blueprint: self.blueprint.map(|blueprint| DbImportBlueprint {
                title: blueprint.title,
                genre: blueprint.genre,
                style: blueprint.style,
                tone: blueprint.tone,
                target_audience: blueprint.target_audience,
                one_sentence_summary: blueprint.one_sentence_summary,
                full_synopsis: blueprint.full_synopsis,
                world_setting: blueprint.world_setting,
                current_version: blueprint.current_version,
                needs_part_outlines: blueprint.needs_part_outlines,
                total_chapters: blueprint.total_chapters,
                chapters_per_part: blueprint.chapters_per_part,
                avatar_svg: blueprint.avatar_svg,
                avatar_animal: blueprint.avatar_animal,
            }),
            blueprint_versions: self
                .blueprint_versions
                .into_iter()
                .map(|version| DbImportBlueprintVersion {
                    version: version.version,
                    change_description: version.change_description,
                    blueprint_data: version.blueprint_data,
                    world_setting_snapshot: version.world_setting_snapshot,
                    characters_snapshot: version.characters_snapshot,
                    relationships_snapshot: version.relationships_snapshot,
                    synopsis_snapshot: version.synopsis_snapshot,
                    created_by: version.created_by,
                })
                .collect(),
            characters: self
                .characters
                .into_iter()
                .map(|character| DbImportCharacter {
                    name: character.name,
                    character_type: character.character_type,
                    gender: character.gender,
                    age: character.age,
                    appearance: character.appearance,
                    personality: character.personality,
                    background: character.background,
                    motivation: character.motivation,
                    arc_summary: character.arc_summary,
                    relationship_to_protagonist: character.relationship_to_protagonist,
                    arc_stages: character.arc_stages,
                })
                .collect(),
            relationships: self
                .relationships
                .into_iter()
                .map(|relationship| DbImportRelationship {
                    relationship_type: relationship.relationship_type,
                    character_a: relationship.character_a,
                    character_b: relationship.character_b,
                    description: relationship.description,
                    strength: relationship.strength,
                    evolution: relationship.evolution,
                })
                .collect(),
            part_outlines: self
                .part_outlines
                .into_iter()
                .map(|part| DbImportPartOutline {
                    old_id: part.old_id,
                    part_number: part.part_number,
                    title: part.title,
                    start_chapter: part.start_chapter,
                    end_chapter: part.end_chapter,
                    summary: part.summary,
                    theme: part.theme,
                    key_events: part.key_events,
                    character_focus: part.character_focus,
                })
                .collect(),
            chapter_outlines: self
                .chapter_outlines
                .into_iter()
                .map(|outline| DbImportChapterOutline {
                    old_id: outline.old_id,
                    old_part_id: outline.old_part_id,
                    chapter_number: outline.chapter_number,
                    title: outline.title,
                    summary: outline.summary,
                    beats: outline.beats,
                    key_scenes: outline.key_scenes,
                    pacing_score: outline.pacing_score,
                    foreshadowing_setup: outline.foreshadowing_setup,
                    foreshadowing_payoff: outline.foreshadowing_payoff,
                    characters_involved: outline.characters_involved,
                    word_count_target: outline.word_count_target,
                    status: outline.status,
                })
                .collect(),
        }
    }
}
