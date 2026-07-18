use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectPartOutline {
    pub old_id: Option<i64>,
    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub summary: Option<String>,
    pub theme: Option<String>,
    pub key_events: Option<Value>,
    pub character_focus: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectChapterOutline {
    pub old_id: Option<i64>,
    pub old_part_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: Option<i32>,
    pub status: String,
}
