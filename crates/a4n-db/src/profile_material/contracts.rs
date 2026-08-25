use a4n_core::models::{ai::generated_image, asset::profile_primary_material};
use sea_orm::{DbErr, FromQueryResult};

#[derive(Debug, thiserror::Error)]
pub enum ProfileMaterialDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

#[derive(Debug, Clone)]
pub struct ProfileMaterialListRows {
    pub images: Vec<generated_image::Model>,
    pub primary_rows: Vec<profile_primary_material::Model>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct ProfileMaterialImageIdentityRow {
    pub id: i64,
    pub image_type: String,
    pub metadata_json: Option<serde_json::Value>,
}
