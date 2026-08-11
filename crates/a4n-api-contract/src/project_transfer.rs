use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PROJECT_FAMILY_PACKAGE_SUFFIX: &str = ".a4n-family.zip";
pub const VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX: &str = ".a4n-video-studio.zip";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExportProjectFamilyDataInput {
    pub root_project_id: i64,
    pub include_derived_projects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExportDerivedProjectDataInput {
    pub derived_project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExportVideoStudioProjectDataInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExportProjectPackageData {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectPackageUploadInput {
    pub file_name: String,
    pub data_base64: String,
}
