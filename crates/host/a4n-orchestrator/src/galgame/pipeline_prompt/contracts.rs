use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelinePromptChapter {
    pub source_snapshot_chapter_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content_hash: String,
    pub content: String,
    pub source_spans: Vec<GalgamePipelinePromptSourceSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelinePromptSourceSpan {
    pub id: String,
    pub text_anchor_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub text: String,
}
