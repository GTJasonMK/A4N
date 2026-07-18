use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::actions::project_transfer) struct ImportPayload {
    pub(super) chapters: Vec<ImportChapter>,
    pub(super) blueprint: Option<ImportBlueprint>,
    pub(super) blueprint_versions: Vec<ImportBlueprintVersion>,
    pub(super) characters: Vec<ImportCharacter>,
    pub(super) relationships: Vec<ImportRelationship>,
    pub(super) part_outlines: Vec<ImportPartOutline>,
    pub(super) chapter_outlines: Vec<ImportChapterOutline>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportChapter {
    pub(super) old_outline_id: Option<i64>,
    pub(super) chapter_number: i32,
    pub(super) title: Option<String>,
    pub(super) content: String,
    pub(super) word_count: i32,
    pub(super) status: String,
    pub(super) summary: Option<String>,
    pub(super) style_tags: Option<Value>,
    pub(super) analysis_data: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportBlueprint {
    pub(super) title: String,
    pub(super) genre: String,
    pub(super) style: Option<String>,
    pub(super) tone: Option<String>,
    pub(super) target_audience: Option<String>,
    pub(super) one_sentence_summary: Option<String>,
    pub(super) full_synopsis: Option<String>,
    pub(super) world_setting: Option<Value>,
    pub(super) current_version: String,
    pub(super) needs_part_outlines: bool,
    pub(super) total_chapters: Option<i32>,
    pub(super) chapters_per_part: Option<i32>,
    pub(super) avatar_svg: Option<String>,
    pub(super) avatar_animal: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportBlueprintVersion {
    pub(super) version: String,
    pub(super) change_description: Option<String>,
    pub(super) blueprint_data: Value,
    pub(super) world_setting_snapshot: Option<Value>,
    pub(super) characters_snapshot: Option<Value>,
    pub(super) relationships_snapshot: Option<Value>,
    pub(super) synopsis_snapshot: Option<String>,
    pub(super) created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportCharacter {
    pub(super) name: String,
    pub(super) character_type: String,
    pub(super) gender: Option<String>,
    pub(super) age: Option<String>,
    pub(super) appearance: Option<String>,
    pub(super) personality: Option<String>,
    pub(super) background: Option<String>,
    pub(super) motivation: Option<String>,
    pub(super) arc_summary: Option<String>,
    pub(super) relationship_to_protagonist: Option<String>,
    pub(super) arc_stages: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportRelationship {
    pub(super) relationship_type: String,
    pub(super) character_a: String,
    pub(super) character_b: String,
    pub(super) description: Option<String>,
    pub(super) strength: Option<i32>,
    pub(super) evolution: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportPartOutline {
    pub(super) old_id: Option<i64>,
    pub(super) part_number: i32,
    pub(super) title: String,
    pub(super) start_chapter: i32,
    pub(super) end_chapter: i32,
    pub(super) summary: Option<String>,
    pub(super) theme: Option<String>,
    pub(super) key_events: Option<Value>,
    pub(super) character_focus: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportChapterOutline {
    pub(super) old_id: Option<i64>,
    pub(super) old_part_id: Option<i64>,
    pub(super) chapter_number: i32,
    pub(super) title: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) beats: Option<Value>,
    pub(super) key_scenes: Option<Value>,
    pub(super) pacing_score: Option<f32>,
    pub(super) foreshadowing_setup: Option<Value>,
    pub(super) foreshadowing_payoff: Option<Value>,
    pub(super) characters_involved: Option<Value>,
    pub(super) word_count_target: Option<i32>,
    pub(super) status: String,
}
