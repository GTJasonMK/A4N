use a4n_db_models::models::{
    character::protagonist_profile,
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ProjectChapterContextError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, Clone)]
pub struct ChapterProfileEvidence {
    pub anchor_id: i64,
    pub chapter_number: i32,
    pub quote: String,
}

#[derive(Debug, Clone)]
pub enum ChapterObjectProfileContext {
    Authored(a4n_db_models::models::asset::object_profile::Model),
    Observed {
        profile_id: i64,
        name: String,
        evidence: Vec<ChapterProfileEvidence>,
    },
}

#[derive(Debug, Clone)]
pub enum ChapterSceneProfileContext {
    Authored(a4n_db_models::models::scene::scene_profile::Model),
    Observed {
        profile_id: i64,
        name: String,
        occurrence: a4n_db_models::models::scene::scene_occurrence::Model,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ChapterGenerationReferenceContextRows {
    pub blueprint: Option<novel_blueprint::Model>,
    pub characters: Vec<blueprint_character::Model>,
    pub relationships: Vec<blueprint_relationship::Model>,
    pub protagonist_profiles: Vec<protagonist_profile::Model>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtagonistProfileCurrentStateUpdate {
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub current_state: Value,
}
