use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataMigrationPreflight {
    pub name: String,
    pub affected_projects: usize,
    pub media_files_to_copy: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMigrationPreflight {
    pub pending_schema_migrations: Vec<String>,
    pub pending_data_migrations: Vec<String>,
    pub migrations: Vec<UpdateDataMigrationPreflight>,
}
