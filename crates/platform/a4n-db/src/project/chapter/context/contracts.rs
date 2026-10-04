use a4n_domain::character::CharacterProfileRecord;
use a4n_domain::object_profile::ObjectProfileRecord;
use a4n_domain::project::{BlueprintCharacterRecord, BlueprintRecord, BlueprintRelationshipRecord};
use a4n_domain::scene_profile::{SceneOccurrenceRecord, SceneProfileRecord};
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
        occurrence: SceneOccurrenceRecord,
        fact_state: Option<Value>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ChapterGenerationReferenceContextRows {
    pub blueprint: Option<BlueprintRecord>,
    pub characters: Vec<BlueprintCharacterRecord>,
    pub relationships: Vec<BlueprintRelationshipRecord>,
    pub protagonist_profiles: Vec<CharacterProfileRecord>,
}
