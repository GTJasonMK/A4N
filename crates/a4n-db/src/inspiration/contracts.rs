use a4n_core::models::{
    chapter::chapter,
    project::{blueprint_character, blueprint_relationship, novel_blueprint, novel_project},
};
use sea_orm::DbErr;

#[derive(Debug, Clone)]
pub struct InspirationBlueprintRows {
    pub blueprint: novel_blueprint::Model,
    pub characters: Vec<blueprint_character::Model>,
    pub relationships: Vec<blueprint_relationship::Model>,
}

#[derive(Debug, Clone)]
pub struct InspirationProjectContextRows {
    pub project: novel_project::Model,
    pub blueprint: Option<InspirationBlueprintRows>,
    pub recent_chapters: Vec<chapter::Model>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspirationExchangeInsertInput {
    pub project_id: i64,
    pub user_input: String,
    pub assistant_output: String,
}

#[derive(Debug, thiserror::Error)]
pub enum InspirationDbError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
