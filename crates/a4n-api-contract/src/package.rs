use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PACKAGE_CONTRACT_VERSION: u32 = 1;
pub const HOST_SCOPE: &str = "a4n-server";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum PackageId {
    Platform,
    NovelManagement,
    Export,
    AiWorkflow,
    VectorRag,
    LineageView,
    Agent,
    Manga,
    Video,
    Galgame,
    Canvas,
    ExternalImport,
}

impl PackageId {
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Platform | Self::NovelManagement | Self::Export)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::NovelManagement => "novel-management",
            Self::Export => "export",
            Self::AiWorkflow => "ai-workflow",
            Self::VectorRag => "vector-rag",
            Self::LineageView => "lineage-view",
            Self::Agent => "agent",
            Self::Manga => "manga",
            Self::Video => "video",
            Self::Galgame => "galgame",
            Self::Canvas => "canvas",
            Self::ExternalImport => "external-import",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "platform" => Some(Self::Platform),
            "novel-management" => Some(Self::NovelManagement),
            "export" => Some(Self::Export),
            "ai-workflow" => Some(Self::AiWorkflow),
            "vector-rag" => Some(Self::VectorRag),
            "lineage-view" => Some(Self::LineageView),
            "agent" => Some(Self::Agent),
            "manga" => Some(Self::Manga),
            "video" => Some(Self::Video),
            "galgame" => Some(Self::Galgame),
            "canvas" => Some(Self::Canvas),
            "external-import" => Some(Self::ExternalImport),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageCommandCategory {
    Normal,
    Generation,
    Maintenance,
}

impl PackageCommandCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Generation => "generation",
            Self::Maintenance => "maintenance",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PackageDefinition {
    pub id: PackageId,
    pub required: bool,
    pub dependencies: Vec<PackageId>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PackageCatalog {
    pub contract_version: u32,
    pub host_scope: String,
    pub packages: Vec<PackageDefinition>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageInstallationState {
    Installed,
    Missing,
    Incompatible,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageHostState {
    Enabled,
    Disabled,
    Enabling,
    Disabling,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageHealth {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageLifecycleAction {
    Enable,
    Disable,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageLifecycleState {
    Pending,
    Draining,
    WaitingExternal,
    Failed,
    Completed,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageReasonCode {
    Allowed,
    NotInstalled,
    Incompatible,
    Disabled,
    Disabling,
    CapabilityUnavailable,
    MaintenanceUnavailable,
    PermissionDenied,
    HostAdminRequired,
    OperationInProgress,
    VersionConflict,
    UnknownCommand,
}

impl PackageReasonCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::NotInstalled => "not_installed",
            Self::Incompatible => "incompatible",
            Self::Disabled => "disabled",
            Self::Disabling => "disabling",
            Self::CapabilityUnavailable => "capability_unavailable",
            Self::MaintenanceUnavailable => "maintenance_unavailable",
            Self::PermissionDenied => "permission_denied",
            Self::HostAdminRequired => "host_admin_required",
            Self::OperationInProgress => "operation_in_progress",
            Self::VersionConflict => "version_conflict",
            Self::UnknownCommand => "unknown_command",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PackageLifecycleSummary {
    pub operation_id: String,
    pub action: PackageLifecycleAction,
    pub state: PackageLifecycleState,
    pub requested_version: Option<String>,
    pub status_version: u64,
    pub reason_code: Option<PackageReasonCode>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageLifecycleRecoveryItem {
    pub operation_id: String,
    pub package_id: String,
    pub action: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PackageOverview {
    pub package_id: PackageId,
    pub required: bool,
    pub installation: PackageInstallationState,
    pub host_state: PackageHostState,
    pub health: PackageHealth,
    pub status_version: u64,
    pub lifecycle: Option<PackageLifecycleSummary>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PackageCommandAvailability {
    pub command: String,
    pub package_id: PackageId,
    pub category: PackageCommandCategory,
    pub available: bool,
    pub reason_code: PackageReasonCode,
    pub status_version: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListPackageCommandAvailabilityInput {
    #[serde(default)]
    pub commands: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartPackageLifecycleInput {
    pub package_id: PackageId,
    pub action: PackageLifecycleAction,
    #[serde(default)]
    pub requested_version: Option<String>,
    #[serde(default)]
    pub expected_status_version: Option<u64>,
    pub operation_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetPackageOperationInput {
    pub operation_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PackagePreferencesSummary {
    pub host_scope: String,
    pub hidden_package_ids: Vec<PackageId>,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdatePackagePreferencesInput {
    pub hidden_package_ids: Vec<PackageId>,
}

/// Host-facing package state read boundary.
///
/// Lifecycle ownership remains with the Host/Orchestrator integration, while
/// Runtime receives only stable package state projections and read decisions.
#[async_trait::async_trait]
pub trait PackageReadApplicationPort: Send + Sync {
    async fn overview(&self) -> Result<Vec<PackageOverview>, a4n_error::AppError>;

    async fn list_command_availability(
        &self,
        user_id: i64,
        input: ListPackageCommandAvailabilityInput,
    ) -> Result<Vec<PackageCommandAvailability>, a4n_error::AppError>;

    async fn command_availability(
        &self,
        user_id: i64,
        command: &str,
    ) -> Result<PackageCommandAvailability, a4n_error::AppError>;

    async fn execution_enabled(&self, package_id: PackageId) -> Result<bool, a4n_error::AppError>;

    fn owned_services(&self, package_id: PackageId) -> Vec<String>;

    async fn services_safe_to_stop(
        &self,
        package_id: PackageId,
    ) -> Result<Vec<String>, a4n_error::AppError>;

    async fn enabled_services(&self) -> Result<Vec<String>, a4n_error::AppError>;

    async fn has_workload(&self, package_id: PackageId) -> Result<bool, a4n_error::AppError>;

    async fn pending_lifecycle_operations(
        &self,
    ) -> Result<Vec<PackageLifecycleRecoveryItem>, a4n_error::AppError>;

    async fn operation(
        &self,
        user_id: i64,
        input: GetPackageOperationInput,
    ) -> Result<PackageLifecycleSummary, a4n_error::AppError>;

    async fn preferences(
        &self,
        user_id: i64,
    ) -> Result<PackagePreferencesSummary, a4n_error::AppError>;
}

/// Host-facing package lifecycle and preference mutation boundary.
#[async_trait::async_trait]
pub trait PackageMutationApplicationPort: Send + Sync {
    async fn initialize_states(&self) -> Result<(), a4n_error::AppError>;

    async fn ensure_execution_enabled(
        &self,
        package_id: PackageId,
    ) -> Result<(), a4n_error::AppError>;

    async fn start_lifecycle(
        &self,
        user_id: i64,
        input: StartPackageLifecycleInput,
    ) -> Result<PackageLifecycleSummary, a4n_error::AppError>;

    async fn advance_lifecycle(
        &self,
        operation_id: &str,
    ) -> Result<PackageLifecycleSummary, a4n_error::AppError>;

    async fn finalize_disable(&self, operation_id: &str) -> Result<(), a4n_error::AppError>;

    async fn update_preferences(
        &self,
        user_id: i64,
        input: UpdatePackagePreferencesInput,
    ) -> Result<PackagePreferencesSummary, a4n_error::AppError>;
}

pub fn package_catalog() -> PackageCatalog {
    PackageCatalog {
        contract_version: PACKAGE_CONTRACT_VERSION,
        host_scope: HOST_SCOPE.to_string(),
        packages: vec![
            package(PackageId::Platform, &[]),
            package(PackageId::NovelManagement, &[PackageId::Platform]),
            package(PackageId::Export, &[PackageId::NovelManagement]),
            package(PackageId::AiWorkflow, &[PackageId::NovelManagement]),
            package(PackageId::VectorRag, &[PackageId::NovelManagement]),
            package(PackageId::LineageView, &[PackageId::NovelManagement]),
            package(PackageId::Agent, &[PackageId::AiWorkflow]),
            package(
                PackageId::Manga,
                &[PackageId::NovelManagement, PackageId::AiWorkflow],
            ),
            package(
                PackageId::Video,
                &[PackageId::NovelManagement, PackageId::AiWorkflow],
            ),
            package(
                PackageId::Galgame,
                &[PackageId::NovelManagement, PackageId::AiWorkflow],
            ),
            package(PackageId::Canvas, &[PackageId::Platform]),
            package(PackageId::ExternalImport, &[PackageId::NovelManagement]),
        ],
    }
}

fn package(id: PackageId, dependencies: &[PackageId]) -> PackageDefinition {
    PackageDefinition {
        id,
        required: id.is_required(),
        dependencies: dependencies.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::{PackageId, package_catalog};

    #[test]
    fn catalog_keeps_three_required_and_nine_optional_packages() {
        let catalog = package_catalog();
        assert_eq!(catalog.packages.len(), 12);
        assert_eq!(
            catalog
                .packages
                .iter()
                .filter(|package| package.required)
                .count(),
            3
        );
        assert!(
            catalog
                .packages
                .iter()
                .all(|package| package.required == package.id.is_required())
        );
        assert_eq!(catalog.packages[1].dependencies, vec![PackageId::Platform]);
    }
}
