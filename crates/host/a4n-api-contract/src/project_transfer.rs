use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROJECT_FAMILY_PACKAGE_SUFFIX: &str = ".a4n-family.zip";
pub const CANVAS_PROJECT_PACKAGE_SUFFIX: &str = ".a4n-canvas.zip";
pub const PROJECT_BUNDLE_PACKAGE_SUFFIX: &str = ".a4n-project.zip";
pub const MEDIA_ASSET_FILE_PATH_FIELD: &str = "file_path";
pub const MEDIA_ASSET_THUMBNAIL_PATH_FIELD: &str = "thumbnail_path";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportProjectBundleDataInput {
    pub project_id: i64,
    pub include_derived_projects: bool,
}

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
pub struct ExportCanvasProjectDataInput {
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

pub type InspectProjectBundleUploadInput = ImportProjectPackageUploadInput;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ProjectBundleProviderSlotReport {
    pub slot_key: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub configuration_fingerprint_sha256: String,
    pub status: String,
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectBundleImportReport {
    pub project_summaries: Vec<crate::project::ProjectSummary>,
    pub conversations_imported: usize,
    pub messages_imported: usize,
    pub workflow_runs_imported: usize,
    pub workflow_runs_marked_interrupted: usize,
    pub generation_runs_imported: usize,
    pub missing_provider_slots: Vec<ProjectBundleProviderSlotReport>,
    pub warnings: Vec<String>,
}

/// 项目包内的媒体文件索引。
///
/// 该类型只描述包合同，不携带 ORM 记录；`original_path` 由 Host 用于读取
/// 已存在的文件，`package_path` 是归档内的稳定路径。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaFileEntry {
    pub owner_kind: String,
    pub owner_id: i64,
    pub field: String,
    pub original_path: String,
    pub package_path: String,
}

/// 已从归档读取的原始 manifest 与媒体字节。
#[derive(Debug)]
pub struct ProjectPackageImportPayload {
    pub manifest: Value,
    pub media_bytes: HashMap<String, Vec<u8>>,
}

/// Host 侧项目包归档输入。
#[derive(Debug)]
pub struct ProjectPackageExportInput {
    pub manifest: Value,
    pub manifest_path: String,
    pub manifest_label: String,
    pub package_label: String,
    pub media_files: Vec<MediaFileEntry>,
}

#[derive(Debug)]
pub struct ExportedProjectPackage {
    pub project_title: String,
    pub package_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct ImportProjectPackageApplicationInput {
    pub user_id: i64,
    pub package_bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectPackageTransferError {
    #[error("{0}")]
    Application(#[from] a4n_error::AppError),
    #[error("{0}")]
    Message(String),
}

impl From<String> for ProjectPackageTransferError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

/// 项目包导入/导出的唯一 Host 能力边界。
///
/// 归档、文件和回滚由 Host 执行；项目包 ORM 模型及业务校验留在 Project owner。
#[async_trait::async_trait]
pub trait ProjectPackageTransferHostPort: Send + Sync {
    fn write_imported_media_file(
        &mut self,
        relative_path: &str,
        bytes: &[u8],
        label: &str,
    ) -> Result<(), String>;

    async fn rollback_imported_media_files(&mut self) -> Result<(), String>;

    async fn rollback_imported_project(&self, user_id: i64, project_id: i64) -> Result<(), String>;

    async fn render_export_package(
        &self,
        input: ProjectPackageExportInput,
    ) -> Result<Vec<u8>, String>;

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
        manifest_path: &str,
        package_label: &str,
        manifest_label: &str,
    ) -> Result<ProjectPackageImportPayload, String>;
}

/// 项目族包的业务应用边界。
#[async_trait::async_trait]
pub trait ProjectFamilyTransferApplicationPort: Send + Sync {
    async fn export_project_family(
        &self,
        input: ExportProjectFamilyPackageApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>;

    async fn export_derived_project(
        &self,
        input: ExportDerivedProjectPackageApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>;

    async fn import_project_family(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<Vec<crate::project::ProjectSummary>, ProjectPackageTransferError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportProjectFamilyPackageApplicationInput {
    pub user_id: i64,
    pub root_project_id: i64,
    pub include_derived_projects: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportDerivedProjectPackageApplicationInput {
    pub user_id: i64,
    pub derived_project_id: i64,
}

/// 独立 Canvas 项目包的业务应用边界。
#[async_trait::async_trait]
pub trait CanvasProjectTransferApplicationPort: Send + Sync {
    async fn export_project(
        &self,
        input: ExportCanvasProjectApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>;

    async fn import_project(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<crate::project::ProjectSummary, ProjectPackageTransferError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportCanvasProjectApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportProjectBundleApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
    pub include_derived_projects: bool,
}

/// Unified project/family/canvas transfer boundary.
#[async_trait::async_trait]
pub trait ProjectBundleTransferApplicationPort: Send + Sync {
    async fn export_bundle(
        &self,
        input: ExportProjectBundleApplicationInput,
        host_port: &dyn ProjectPackageTransferHostPort,
    ) -> Result<ExportedProjectPackage, ProjectPackageTransferError>;

    async fn inspect_bundle(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<ProjectBundleImportReport, ProjectPackageTransferError>;

    async fn import_bundle(
        &self,
        input: ImportProjectPackageApplicationInput,
        host_port: &mut dyn ProjectPackageTransferHostPort,
    ) -> Result<ProjectBundleImportReport, ProjectPackageTransferError>;
}
