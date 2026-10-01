use a4n_db_models::models::{
    character::protagonist_profile,
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use a4n_domain::{object_profile::ObjectProfileRecord, scene_profile::SceneProfileRecord};
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
    Authored(ObjectProfileRecord),
    Observed {
        profile_id: i64,
        name: String,
        evidence: Vec<ChapterProfileEvidence>,
        fact_state: Option<Value>,
    },
}

#[derive(Debug, Clone)]
pub enum ChapterSceneProfileContext {
    Authored(SceneProfileRecord),
    Observed {
        profile_id: i64,
        name: String,
        occurrence: a4n_db_models::models::scene::scene_occurrence::Model,
        fact_state: Option<Value>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ChapterGenerationReferenceContextRows {
    pub blueprint: Option<novel_blueprint::Model>,
    pub characters: Vec<blueprint_character::Model>,
    pub relationships: Vec<blueprint_relationship::Model>,
    pub protagonist_profiles: Vec<protagonist_profile::Model>,
}
