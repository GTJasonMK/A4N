use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, novel_blueprint, novel_project},
};
use chrono::NaiveDateTime;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ImportReverseDbError {
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
}

impl From<crate::project::BlueprintSyncError> for ImportReverseDbError {
    fn from(error: crate::project::BlueprintSyncError) -> Self {
        match error {
            crate::project::BlueprintSyncError::InvalidArgument(message) => {
                Self::InvalidArgument(message)
            }
            crate::project::BlueprintSyncError::Database(error) => Self::Database(error),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReverseContextRows {
    pub project: novel_project::Model,
    pub chapters: Vec<chapter::Model>,
    pub chapter_outlines: Vec<chapter_outline::Model>,
    pub part_outlines: Vec<part_outline::Model>,
    pub blueprint_context: Option<ReverseBlueprintContextRows>,
}

#[derive(Debug, Clone)]
pub struct ReverseBlueprintContextRows {
    pub foreshadowing: Vec<foreshadowing_index::Model>,
    pub existing_blueprint: Option<novel_blueprint::Model>,
    pub blueprint_characters: Vec<blueprint_character::Model>,
    pub protagonist_profiles: Vec<protagonist_profile::Model>,
}

#[derive(Debug, Clone)]
pub struct ReverseOutlineApplyRow {
    pub chapter_number: i32,
    pub title: String,
    pub summary: String,
    pub word_count_target: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ReversePartOutlineApplyRow {
    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub summary: String,
    pub theme: String,
    pub key_events: Value,
    pub character_focus: Value,
}

#[derive(Debug, Clone)]
pub struct ReverseBlueprintApplyPlanRecord {
    pub project_total_chapters: Option<i32>,
    pub existing_blueprint_total_chapters: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ReverseBlueprintApplyRecord {
    pub applied_at: NaiveDateTime,
    pub title: String,
    pub genre: String,
    pub style: String,
    pub tone: String,
    pub target_audience: String,
    pub one_sentence_summary: String,
    pub full_synopsis: String,
    pub world_setting: Value,
    pub total_chapters: Option<i32>,
    pub needs_part_outlines: bool,
    pub chapters_per_part: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ReverseBlueprintApplyDraft {
    pub title: String,
    pub genre: String,
    pub style: String,
    pub tone: String,
    pub target_audience: String,
    pub one_sentence_summary: String,
    pub full_synopsis: String,
    pub world_setting: Value,
    pub incoming_total_chapters: Option<i32>,
    pub incoming_needs_part_outlines: Option<bool>,
    pub characters: Vec<crate::project::BlueprintCharacterDraft>,
    pub relationships: Vec<crate::project::BlueprintRelationshipDraft>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReverseBlueprintApplyResult {
    pub character_count: usize,
    pub relationship_count: usize,
}
