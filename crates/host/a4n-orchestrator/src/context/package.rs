use std::collections::HashSet;

use a4n_api_contract::command_manifest::{API_COMMANDS, CommandDescriptor, command_descriptor};
use a4n_api_contract::package::{
    PackageCommandAvailability, PackageCommandCategory, PackageDefinition, PackageHealth,
    PackageHostState, PackageId, PackageInstallationState, PackageLifecycleAction,
    PackageLifecycleState, PackageLifecycleSummary, PackageOverview, PackagePreferencesSummary,
    PackageReasonCode, StartPackageLifecycleInput, UpdatePackagePreferencesInput, package_catalog,
};
use a4n_domain::package::HostPackageOperationRecord;
use a4n_error::AppError;
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, EXPORT_SERVICE_NAME, HealthStatus, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, LLM_SERVICE_NAME, PROMPT_SERVICE_NAME, RAG_SERVICE_NAME,
    VIDEO_SERVICE_NAME,
};

use super::OrchestratorContext;
use crate::admin::AdminPermission;

mod mutations;

impl OrchestratorContext {
    pub(crate) async fn package_overview(&self) -> Result<Vec<PackageOverview>, AppError> {
        let states = a4n_db::package::list_host_package_states(self.database.executor()).await?;
        let mut overview = Vec::new();
        for definition in package_catalog().packages {
            let state = states
                .iter()
                .find(|state| state.package_id == definition.id.as_str());
            let lifecycle = match state {
                Some(state) => a4n_db::package::latest_package_operation(
                    self.database.executor(),
                    &state.package_id,
                )
                .await?
                .map(package_lifecycle_summary)
                .transpose()?,
                None => None,
            };
            let (installation, host_state, status_version) = match state {
                Some(state) => (
                    installation_state(&state.installation_state),
                    host_state(&state.host_state)?,
                    parse_status_version(state.status_version, "读取功能包状态")?,
                ),
                None => (
                    PackageInstallationState::Unknown,
                    PackageHostState::Disabled,
                    0,
                ),
            };
            overview.push(PackageOverview {
                package_id: definition.id,
                required: definition.required,
                installation,
                host_state,
                health: self.package_health(definition.id).await,
                status_version,
                lifecycle,
            });
        }
        Ok(overview)
    }

    pub(crate) async fn package_command_availability(
        &self,
        user_id: i64,
        command: &str,
    ) -> Result<PackageCommandAvailability, AppError> {
        let descriptor = command_descriptor(command)
            .ok_or_else(|| AppError::NotFound(format!("未知 Host API 命令: {command}")))?;
        self.package_command_availability_for_descriptor(user_id, &descriptor)
            .await
    }

    pub(crate) async fn list_package_command_availability(
        &self,
        user_id: i64,
        commands: Option<&[String]>,
    ) -> Result<Vec<PackageCommandAvailability>, AppError> {
        let command_names = commands
            .map(|commands| commands.to_vec())
            .unwrap_or_else(|| {
                API_COMMANDS
                    .iter()
                    .map(|command| (*command).to_string())
                    .collect()
            });
        let mut result = Vec::with_capacity(command_names.len());
        for command in command_names {
            let Some(descriptor) = command_descriptor(&command) else {
                result.push(PackageCommandAvailability {
                    command,
                    package_id: PackageId::Platform,
                    category: PackageCommandCategory::Normal,
                    available: false,
                    reason_code: PackageReasonCode::UnknownCommand,
                    status_version: 0,
                });
                continue;
            };
            result.push(
                self.package_command_availability_for_descriptor(user_id, &descriptor)
                    .await?,
            );
        }
        Ok(result)
    }

    pub(crate) async fn package_execution_enabled(
        &self,
        package_id: PackageId,
    ) -> Result<bool, AppError> {
        Ok(self
            .package_execution_block_reason(package_id)
            .await?
            .is_none())
    }

    pub(crate) async fn ensure_package_execution_enabled(
        &self,
        package_id: PackageId,
    ) -> Result<(), AppError> {
        if let Some(reason_code) = self.package_execution_block_reason(package_id).await? {
            return Err(package_lifecycle_error(
                package_id,
                reason_code,
                "功能包执行准入已关闭",
            ));
        }
        Ok(())
    }

    async fn package_execution_block_reason(
        &self,
        package_id: PackageId,
    ) -> Result<Option<PackageReasonCode>, AppError> {
        let Some(state) =
            a4n_db::package::host_package_state(self.database.executor(), package_id.as_str())
                .await?
        else {
            return Ok(Some(PackageReasonCode::NotInstalled));
        };
        if installation_state(&state.installation_state) != PackageInstallationState::Installed {
            return Ok(Some(match installation_state(&state.installation_state) {
                PackageInstallationState::Incompatible => PackageReasonCode::Incompatible,
                PackageInstallationState::Missing | PackageInstallationState::Unknown => {
                    PackageReasonCode::NotInstalled
                }
                PackageInstallationState::Installed => unreachable!(),
            }));
        }
        Ok(match host_state(&state.host_state)? {
            PackageHostState::Enabled => {
                if self.dependencies_available(package_id).await? {
                    None
                } else {
                    Some(PackageReasonCode::CapabilityUnavailable)
                }
            }
            PackageHostState::Disabled => Some(PackageReasonCode::Disabled),
            PackageHostState::Disabling => Some(PackageReasonCode::Disabling),
            PackageHostState::Enabling => Some(PackageReasonCode::OperationInProgress),
        })
    }

    pub(crate) async fn pending_package_lifecycle_operations(
        &self,
    ) -> Result<Vec<HostPackageOperationRecord>, AppError> {
        a4n_db::package::pending_package_operations(self.database.executor()).await
    }

    pub(crate) async fn enabled_package_service_names(&self) -> Result<Vec<String>, AppError> {
        let states = a4n_db::package::list_host_package_states(self.database.executor()).await?;
        let mut services = HashSet::new();
        for state in states {
            if state.installation_state != a4n_db::package::INSTALLATION_INSTALLED
                || state.host_state != a4n_db::package::HOST_STATE_ENABLED
            {
                continue;
            }
            let Some(package_id) = PackageId::parse(&state.package_id) else {
                continue;
            };
            services.extend(services_for_package(package_id).iter().copied());
        }
        let mut services = services.into_iter().map(str::to_string).collect::<Vec<_>>();
        services.sort();
        Ok(services)
    }

    pub(crate) async fn package_workload_snapshot(
        &self,
        package_id: &str,
    ) -> Result<a4n_db::package::PackageWorkloadSnapshot, AppError> {
        a4n_db::package::package_workload_snapshot(self.database.executor(), package_id).await
    }

    /// 返回可以安全停止的 managed service。任何其他包处于未知、过渡或
    /// 明确启用状态时都保留共享 service，避免停用一个包误杀另一个包。
    pub(crate) async fn package_services_safe_to_stop(
        &self,
        package_id: PackageId,
    ) -> Result<Vec<String>, AppError> {
        let target_services = services_for_package(package_id);
        if target_services.is_empty() {
            return Ok(Vec::new());
        }
        let states = a4n_db::package::list_host_package_states(self.database.executor()).await?;
        let catalog = package_catalog();
        let safe_services = target_services
            .iter()
            .filter(|service| {
                catalog
                    .packages
                    .iter()
                    .filter(|definition| definition.id != package_id)
                    .all(|definition| {
                        if !services_for_package(definition.id).contains(service) {
                            return true;
                        }
                        let Some(state) = states
                            .iter()
                            .find(|state| state.package_id == definition.id.as_str())
                        else {
                            return false;
                        };
                        matches!(
                            (state.installation_state.as_str(), state.host_state.as_str()),
                            (
                                a4n_db::package::INSTALLATION_MISSING
                                    | a4n_db::package::INSTALLATION_INCOMPATIBLE,
                                _
                            ) | (_, a4n_db::package::HOST_STATE_DISABLED)
                        )
                    })
            })
            .map(|service| (*service).to_string())
            .collect();
        Ok(safe_services)
    }

    /// 返回该包直接拥有的 service。生命周期进程装配由 Runtime 执行，
    /// 但 service 归属必须继续由包策略唯一维护。
    pub(crate) fn package_service_names(&self, package_id: PackageId) -> Vec<String> {
        services_for_package(package_id)
            .iter()
            .map(|service| (*service).to_string())
            .collect()
    }

    pub(crate) async fn get_package_operation(
        &self,
        user_id: i64,
        operation_id: &str,
    ) -> Result<PackageLifecycleSummary, AppError> {
        self.require_admin_permission(user_id, AdminPermission::ManageSettings)
            .await?;
        let operation = a4n_db::package::package_operation(self.database.executor(), operation_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))?;
        package_lifecycle_summary(operation)
    }

    pub(crate) async fn package_preferences(
        &self,
        user_id: i64,
    ) -> Result<PackagePreferencesSummary, AppError> {
        let hidden_package_ids =
            a4n_db::package::package_preference(self.database.executor(), user_id)
                .await?
                .map(|preference| parse_hidden_package_ids(preference.hidden_package_ids))
                .transpose()?
                .unwrap_or_default();
        Ok(PackagePreferencesSummary {
            host_scope: a4n_api_contract::package::HOST_SCOPE.to_string(),
            hidden_package_ids,
        })
    }

    async fn package_command_availability_for_descriptor(
        &self,
        user_id: i64,
        descriptor: &CommandDescriptor,
    ) -> Result<PackageCommandAvailability, AppError> {
        let state = a4n_db::package::host_package_state(
            self.database.executor(),
            descriptor.package_id.as_str(),
        )
        .await?;
        let status_version = state
            .as_ref()
            .map(|state| parse_status_version(state.status_version, "读取功能包命令状态"))
            .transpose()?
            .unwrap_or(0);
        let mut reason_code = PackageReasonCode::Allowed;
        let mut available = true;
        let read_or_maintenance = descriptor.effect == "read"
            || descriptor.package_category == PackageCommandCategory::Maintenance;

        if descriptor.auth == "admin" && !self.is_active_admin_user(user_id).await? {
            available = false;
            reason_code = PackageReasonCode::HostAdminRequired;
        } else if let Some(state) = state {
            match installation_state(&state.installation_state) {
                PackageInstallationState::Installed => {}
                PackageInstallationState::Incompatible => {
                    available = false;
                    reason_code = PackageReasonCode::Incompatible;
                }
                PackageInstallationState::Missing | PackageInstallationState::Unknown => {
                    available = false;
                    reason_code = PackageReasonCode::NotInstalled;
                }
            }
            if available && !read_or_maintenance {
                match host_state(&state.host_state)? {
                    PackageHostState::Enabled => {}
                    PackageHostState::Disabled => {
                        available = false;
                        reason_code = PackageReasonCode::Disabled;
                    }
                    PackageHostState::Disabling => {
                        available = false;
                        reason_code = PackageReasonCode::Disabling;
                    }
                    PackageHostState::Enabling => {
                        available = false;
                        reason_code = PackageReasonCode::OperationInProgress;
                    }
                }
            }
        } else {
            available = false;
            reason_code = PackageReasonCode::NotInstalled;
        }

        if available && !read_or_maintenance {
            let dependencies_available = self.dependencies_available(descriptor.package_id).await?;
            if !dependencies_available || !self.required_capabilities_available(descriptor).await {
                available = false;
                reason_code = PackageReasonCode::CapabilityUnavailable;
            }
        }
        Ok(PackageCommandAvailability {
            command: descriptor.name.clone(),
            package_id: descriptor.package_id,
            category: descriptor.package_category,
            available,
            reason_code,
            status_version,
        })
    }

    async fn ensure_dependencies_enabled(
        &self,
        definition: &PackageDefinition,
    ) -> Result<(), AppError> {
        if !self.dependencies_available(definition.id).await? {
            return Err(package_lifecycle_error(
                definition.id,
                PackageReasonCode::CapabilityUnavailable,
                "功能包依赖未安装或未启用",
            ));
        }
        Ok(())
    }

    async fn dependencies_available(&self, package_id: PackageId) -> Result<bool, AppError> {
        let definitions = package_catalog().packages;
        let Some(definition) = definitions
            .iter()
            .find(|definition| definition.id == package_id)
        else {
            return Ok(false);
        };
        for dependency in &definition.dependencies {
            let Some(state) =
                a4n_db::package::host_package_state(self.database.executor(), dependency.as_str())
                    .await?
            else {
                return Ok(false);
            };
            if installation_state(&state.installation_state) != PackageInstallationState::Installed
                || host_state(&state.host_state)? != PackageHostState::Enabled
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn required_capabilities_available(&self, descriptor: &CommandDescriptor) -> bool {
        for capability in &descriptor.required_capabilities {
            let Some(service) = service_for_capability(capability) else {
                return false;
            };
            match self.service_manager.health(service).await {
                Ok(health) if health.status == HealthStatus::Healthy => {}
                _ => return false,
            }
        }
        true
    }

    async fn package_health(&self, package_id: PackageId) -> PackageHealth {
        match self.dependencies_available(package_id).await {
            Ok(true) => {}
            Ok(false) => return PackageHealth::Unhealthy,
            Err(_) => return PackageHealth::Unknown,
        }
        let services = services_for_package(package_id);
        if services.is_empty() {
            return PackageHealth::Healthy;
        }
        let mut unknown = false;
        for service in services {
            match self.service_manager.health(service).await {
                Ok(health) if health.status == HealthStatus::Healthy => {}
                Ok(_) => return PackageHealth::Unhealthy,
                Err(_) => unknown = true,
            }
        }
        if unknown {
            PackageHealth::Unknown
        } else {
            PackageHealth::Healthy
        }
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::package::PackageReadApplicationPort for OrchestratorContext {
    async fn overview(&self) -> Result<Vec<a4n_api_contract::package::PackageOverview>, AppError> {
        self.package_overview().await
    }

    async fn list_command_availability(
        &self,
        user_id: i64,
        input: a4n_api_contract::package::ListPackageCommandAvailabilityInput,
    ) -> Result<Vec<a4n_api_contract::package::PackageCommandAvailability>, AppError> {
        self.list_package_command_availability(user_id, input.commands.as_deref())
            .await
    }

    async fn command_availability(
        &self,
        user_id: i64,
        command: &str,
    ) -> Result<a4n_api_contract::package::PackageCommandAvailability, AppError> {
        self.package_command_availability(user_id, command).await
    }

    async fn execution_enabled(&self, package_id: PackageId) -> Result<bool, AppError> {
        self.package_execution_enabled(package_id).await
    }

    fn owned_services(&self, package_id: PackageId) -> Vec<String> {
        self.package_service_names(package_id)
    }

    async fn services_safe_to_stop(&self, package_id: PackageId) -> Result<Vec<String>, AppError> {
        self.package_services_safe_to_stop(package_id).await
    }

    async fn enabled_services(&self) -> Result<Vec<String>, AppError> {
        self.enabled_package_service_names().await
    }

    async fn has_workload(&self, package_id: PackageId) -> Result<bool, AppError> {
        Ok(!self
            .package_workload_snapshot(package_id.as_str())
            .await?
            .is_empty())
    }

    async fn pending_lifecycle_operations(
        &self,
    ) -> Result<Vec<a4n_api_contract::package::PackageLifecycleRecoveryItem>, AppError> {
        Ok(self
            .pending_package_lifecycle_operations()
            .await?
            .into_iter()
            .map(
                |operation| a4n_api_contract::package::PackageLifecycleRecoveryItem {
                    operation_id: operation.operation_id,
                    package_id: operation.package_id,
                    action: operation.action,
                },
            )
            .collect())
    }

    async fn operation(
        &self,
        user_id: i64,
        input: a4n_api_contract::package::GetPackageOperationInput,
    ) -> Result<a4n_api_contract::package::PackageLifecycleSummary, AppError> {
        self.get_package_operation(user_id, &input.operation_id)
            .await
    }

    async fn preferences(
        &self,
        user_id: i64,
    ) -> Result<a4n_api_contract::package::PackagePreferencesSummary, AppError> {
        self.package_preferences(user_id).await
    }
}

fn package_definition(package_id: PackageId) -> Result<PackageDefinition, AppError> {
    package_catalog()
        .packages
        .into_iter()
        .find(|package| package.id == package_id)
        .ok_or_else(|| AppError::NotFound(format!("未知功能包: {}", package_id.as_str())))
}

fn package_lifecycle_error(
    package_id: PackageId,
    reason_code: PackageReasonCode,
    message: &str,
) -> AppError {
    AppError::InvalidState {
        current: format!(
            "package_id={} reason_code={} {message}",
            package_id.as_str(),
            reason_code.as_str()
        ),
        operation: "功能包生命周期操作".to_string(),
    }
}

fn lifecycle_action(action: &PackageLifecycleAction) -> &'static str {
    match action {
        PackageLifecycleAction::Enable => "enable",
        PackageLifecycleAction::Disable => "disable",
    }
}

fn installation_state(value: &str) -> PackageInstallationState {
    match value {
        a4n_db::package::INSTALLATION_INSTALLED => PackageInstallationState::Installed,
        a4n_db::package::INSTALLATION_MISSING => PackageInstallationState::Missing,
        a4n_db::package::INSTALLATION_INCOMPATIBLE => PackageInstallationState::Incompatible,
        _ => PackageInstallationState::Unknown,
    }
}

fn host_state(value: &str) -> Result<PackageHostState, AppError> {
    match value {
        a4n_db::package::HOST_STATE_ENABLED => Ok(PackageHostState::Enabled),
        a4n_db::package::HOST_STATE_DISABLED => Ok(PackageHostState::Disabled),
        a4n_db::package::HOST_STATE_ENABLING => Ok(PackageHostState::Enabling),
        a4n_db::package::HOST_STATE_DISABLING => Ok(PackageHostState::Disabling),
        other => Err(AppError::InvalidState {
            current: format!("未知功能包 Host 状态: {other}"),
            operation: "读取功能包状态".to_string(),
        }),
    }
}

fn parse_status_version(value: i64, operation: &str) -> Result<u64, AppError> {
    u64::try_from(value).map_err(|_| AppError::InvalidState {
        current: format!("功能包状态版本无效: {value}"),
        operation: operation.to_string(),
    })
}

fn package_lifecycle_summary(
    operation: HostPackageOperationRecord,
) -> Result<PackageLifecycleSummary, AppError> {
    let action = match operation.action.as_str() {
        "enable" => PackageLifecycleAction::Enable,
        "disable" => PackageLifecycleAction::Disable,
        other => {
            return Err(AppError::InvalidState {
                current: format!("未知功能包操作动作: {other}"),
                operation: "读取功能包操作".to_string(),
            });
        }
    };
    let state = match operation.state.as_str() {
        a4n_db::package::OPERATION_PENDING => PackageLifecycleState::Pending,
        a4n_db::package::OPERATION_DRAINING => PackageLifecycleState::Draining,
        a4n_db::package::OPERATION_WAITING_EXTERNAL => PackageLifecycleState::WaitingExternal,
        a4n_db::package::OPERATION_FAILED => PackageLifecycleState::Failed,
        a4n_db::package::OPERATION_COMPLETED => PackageLifecycleState::Completed,
        other => {
            return Err(AppError::InvalidState {
                current: format!("未知功能包操作状态: {other}"),
                operation: "读取功能包操作".to_string(),
            });
        }
    };
    Ok(PackageLifecycleSummary {
        operation_id: operation.operation_id,
        action,
        state,
        requested_version: operation.requested_version,
        status_version: parse_status_version(operation.status_version, "读取功能包操作")?,
        reason_code: operation
            .reason_code
            .as_deref()
            .map(parse_reason_code)
            .transpose()?,
        error: operation.error,
    })
}

fn parse_reason_code(value: &str) -> Result<PackageReasonCode, AppError> {
    [
        PackageReasonCode::Allowed,
        PackageReasonCode::NotInstalled,
        PackageReasonCode::Incompatible,
        PackageReasonCode::Disabled,
        PackageReasonCode::Disabling,
        PackageReasonCode::CapabilityUnavailable,
        PackageReasonCode::MaintenanceUnavailable,
        PackageReasonCode::PermissionDenied,
        PackageReasonCode::HostAdminRequired,
        PackageReasonCode::OperationInProgress,
        PackageReasonCode::VersionConflict,
        PackageReasonCode::UnknownCommand,
    ]
    .into_iter()
    .find(|reason| reason.as_str() == value)
    .ok_or_else(|| AppError::InvalidState {
        current: format!("未知功能包原因码: {value}"),
        operation: "读取功能包状态".to_string(),
    })
}

fn parse_hidden_package_ids(value: serde_json::Value) -> Result<Vec<PackageId>, AppError> {
    let values = value.as_array().ok_or_else(|| AppError::InvalidState {
        current: "功能包偏好不是 JSON 数组".to_string(),
        operation: "读取功能包偏好".to_string(),
    })?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .and_then(PackageId::parse)
                .ok_or_else(|| AppError::InvalidState {
                    current: "功能包偏好包含未知 ID".to_string(),
                    operation: "读取功能包偏好".to_string(),
                })
        })
        .collect()
}

fn service_for_capability(capability: &str) -> Option<&'static str> {
    match capability {
        "prompt" => Some(PROMPT_SERVICE_NAME),
        "llm" => Some(LLM_SERVICE_NAME),
        "embedding" => Some(EMBEDDING_SERVICE_NAME),
        "rag" => Some(RAG_SERVICE_NAME),
        "image" => Some(IMAGE_SERVICE_NAME),
        "video" => Some(VIDEO_SERVICE_NAME),
        "import" => Some(IMPORT_SERVICE_NAME),
        "export" => Some(EXPORT_SERVICE_NAME),
        _ => None,
    }
}

fn services_for_package(package_id: PackageId) -> &'static [&'static str] {
    match package_id {
        PackageId::AiWorkflow => &[PROMPT_SERVICE_NAME, LLM_SERVICE_NAME],
        PackageId::VectorRag => &[EMBEDDING_SERVICE_NAME, RAG_SERVICE_NAME],
        PackageId::Manga => &[IMAGE_SERVICE_NAME],
        PackageId::Video => &[VIDEO_SERVICE_NAME],
        PackageId::Galgame => &[IMAGE_SERVICE_NAME, VIDEO_SERVICE_NAME],
        PackageId::ExternalImport => &[IMPORT_SERVICE_NAME],
        PackageId::Export => &[EXPORT_SERVICE_NAME],
        PackageId::Platform
        | PackageId::NovelManagement
        | PackageId::LineageView
        | PackageId::Agent
        | PackageId::Canvas => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db::package::{
        ensure_host_package_states, host_package_state, start_package_lifecycle,
    };
    use a4n_migration::test_support::connect_test_database;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn package_state_parsing_rejects_invalid_persisted_values() {
        assert_eq!(
            host_state(a4n_db::package::HOST_STATE_ENABLED).unwrap(),
            PackageHostState::Enabled
        );
        assert!(host_state("unknown").is_err());
        assert!(parse_status_version(-1, "test").is_err());
    }

    #[tokio::test]
    async fn package_execution_admission_rejects_a_package_being_disabled() {
        let database = Arc::new(connect_test_database().await);
        let context = crate::test_support::TestOrchestratorBuilder::new(Arc::clone(&database))
            .build()
            .await;
        ensure_host_package_states(
            database.as_ref(),
            &[
                PackageId::Agent.as_str().to_string(),
                PackageId::AiWorkflow.as_str().to_string(),
            ],
        )
        .await
        .expect("初始化 Agent 功能包状态");

        context
            .ensure_package_execution_enabled(PackageId::Agent)
            .await
            .expect("启用中的 Agent 应允许执行");

        let state = host_package_state(database.as_ref(), PackageId::Agent.as_str())
            .await
            .expect("读取 Agent 功能包状态")
            .expect("Agent 功能包状态必须存在");
        start_package_lifecycle(
            database.as_ref(),
            &Uuid::new_v4().to_string(),
            PackageId::Agent.as_str(),
            "disable",
            None,
            7,
            state.status_version,
        )
        .await
        .expect("创建 Agent 停用操作");

        let error = context
            .ensure_package_execution_enabled(PackageId::Agent)
            .await
            .expect_err("停用中的 Agent 不得创建新执行");
        assert!(
            error
                .to_string()
                .contains("package_id=agent reason_code=disabling")
        );
    }

    #[tokio::test]
    async fn package_execution_admission_rejects_a_disabled_dependency() {
        let database = Arc::new(connect_test_database().await);
        let context = crate::test_support::TestOrchestratorBuilder::new(Arc::clone(&database))
            .build()
            .await;
        ensure_host_package_states(
            database.as_ref(),
            &[
                PackageId::Agent.as_str().to_string(),
                PackageId::AiWorkflow.as_str().to_string(),
            ],
        )
        .await
        .expect("初始化 Agent 及其依赖功能包状态");

        let dependency = host_package_state(database.as_ref(), PackageId::AiWorkflow.as_str())
            .await
            .expect("读取 AI workflow 功能包状态")
            .expect("AI workflow 功能包状态必须存在");
        start_package_lifecycle(
            database.as_ref(),
            &Uuid::new_v4().to_string(),
            PackageId::AiWorkflow.as_str(),
            "disable",
            None,
            7,
            dependency.status_version,
        )
        .await
        .expect("创建 AI workflow 停用操作");

        let error = context
            .ensure_package_execution_enabled(PackageId::Agent)
            .await
            .expect_err("依赖停用中的 Agent 不得创建新执行");
        assert!(
            error
                .to_string()
                .contains("package_id=agent reason_code=capability_unavailable")
        );
    }
}
