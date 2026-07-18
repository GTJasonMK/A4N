use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum CharacterProfileSeedError {
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error("{0}")]
    Internal(String),
}

pub(super) struct BlueprintProfileSeed {
    pub(super) base_attributes: Value,
    pub(super) character_arc: Option<Value>,
}
