use a4n_domain::profile_material::{ProfileMaterialImageRecord, ProfilePrimaryMaterialRecord};
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
    pub images: Vec<ProfileMaterialImageRecord>,
    pub primary_rows: Vec<ProfilePrimaryMaterialRecord>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct ProfileMaterialImageIdentityRow {
    pub id: i64,
    pub image_type: String,
    pub metadata_json: Option<serde_json::Value>,
}
