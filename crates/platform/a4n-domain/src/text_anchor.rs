//! Pure text-anchor evidence values.

use chrono::NaiveDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAnchorRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_kind: String,
    pub source_id: i64,
    pub paragraph_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub quote: String,
    pub quote_hash: String,
    pub source_version_hash: String,
    pub created_at: NaiveDateTime,
}

impl crate::source_spans::SourceTextAnchor for TextAnchorRecord {
    fn source_anchor_id(&self) -> i64 {
        self.id
    }
    fn source_anchor_chapter_id(&self) -> Option<i64> {
        self.source_chapter_id
    }
    fn source_anchor_quote(&self) -> &str {
        &self.quote
    }
    fn source_anchor_char_range(&self) -> (i32, i32) {
        (self.start_offset, self.end_offset)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchorProfileKind {
    Character,
    Object,
    Scene,
}

impl TextAnchorProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Object => "object",
            Self::Scene => "scene",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextAnchorEvidenceInput {
    pub text_anchor_id: i64,
    pub evidence_role: String,
}
