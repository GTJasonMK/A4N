use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectFullUploadInput {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportProjectFamilyDataInput {
    pub root_project_id: i64,
    pub include_derived_projects: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportDerivedProjectDataInput {
    pub derived_project_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportVideoStudioProjectDataInput {
    pub project_id: i64,
}

#[derive(Debug, Serialize)]
pub struct ExportProjectPackageData {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectFamilyUploadInput {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportVideoStudioProjectUploadInput {
    pub file_name: String,
    pub data_base64: String,
}
