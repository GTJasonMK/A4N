use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportChapterOutlineUpsertInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub title: String,
    pub summary: String,
    pub key_scenes: Option<Value>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportPartOutlineUpsertInput {
    pub project_id: i64,
    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub summary: String,
    pub theme: String,
    pub key_events: Value,
    pub character_focus: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportBlueprintUpsertInput {
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub one_sentence_summary: String,
    pub full_synopsis: String,
    pub world_setting: Value,
    pub total_chapters: i32,
    pub needs_part_outlines: bool,
    pub chapters_per_part: Option<i32>,
}
