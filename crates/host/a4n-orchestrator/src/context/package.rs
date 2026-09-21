use std::collections::HashSet;

use a4n_api_contract::command_manifest::{API_COMMANDS, CommandDescriptor, command_descriptor};
use a4n_api_contract::package::{
    PackageCommandAvailability, PackageCommandCategory, PackageDefinition, PackageHealth,
    PackageHostState, PackageId, PackageInstallationState, PackageLifecycleAction,
    PackageLifecycleState, PackageLifecycleSummary, PackageOverview, PackagePreferencesSummary,
    PackageReasonCode, StartPackageLifecycleInput, UpdatePackagePreferencesInput, package_catalog,
};
use a4n_error::AppError;
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, EXPORT_SERVICE_NAME, HealthStatus, IMAGE_SERVICE_NAME,
    IMPORT_SERVICE_NAME, LLM_SERVICE_NAME, PROMPT_SERVICE_NAME, RAG_SERVICE_NAME,
    VIDEO_SERVICE_NAME,
};

use super::OrchestratorContext;
use crate::admin::AdminPermission;

impl OrchestratorContext {
    pub(crate) async fn initialize_package_states(&self) -> Result<(), AppError> {
        let catalog = package_catalog();
        let package_ids = catalog
            .packages
            .iter()
            .map(|package| package.id.as_str().to_string())
            .collect::<Vec<_>>();
        a4n_db::package::ensure_host_package_states(self.database.connection(), &package_ids)
            .await?;
        for package in catalog.packages {
            let installed = services_for_package(package.id).iter().all(|service| {
                self.service_manager
                    .config(service)
                    .map(|config| config.mode != a4n_service_manager::ServiceMode::Disabled)
                    .unwrap_or(false)
            });
            a4n_db::package::reconcile_package_installation_state(
                self.database.connection(),
                package.id.as_str(),
                if installed {
                    a4n_db::package::INSTALLATION_INSTALLED
                } else {
                    a4n_db::package::INSTALLATION_MISSING
                },
            )
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn package_overview(&self) -> Result<Vec<PackageOverview>, AppError> {
        let states = a4n_db::package::list_host_package_states(self.database.connection()).await?;
        let mut overview = Vec::new();
        for definition in package_catalog().packages {
            let state = states
                .iter()
                .find(|state| state.package_id == definition.id.as_str());
            let lifecycle = match state {
                Some(state) => a4n_db::package::latest_package_operation(
                    self.database.connection(),
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

    pub(crate) async fn start_package_lifecycle(
        &self,
        user_id: i64,
        input: StartPackageLifecycleInput,
    ) -> Result<PackageLifecycleSummary, AppError> {
        self.require_admin_permission(user_id, AdminPermission::ManageSettings)
            .await?;
        let action = lifecycle_action(&input.action);
        let requested_version = input
            .expected_status_version
            .map(|version| {
                i64::try_from(version)
                    .map_err(|_| AppError::InvalidArgument("expectedStatusVersion 超出范围".into()))
            })
            .transpose()?;
        let expected_version = requested_version.unwrap_or(i64::MAX);
        if a4n_db::package::package_operation(self.database.connection(), &input.operation_id)
            .await?
            .is_some()
        {
            let operation = a4n_db::package::start_package_lifecycle(
                self.database.connection(),
                &input.operation_id,
                input.package_id.as_str(),
                action,
                input.requested_version.clone(),
                user_id,
                expected_version,
            )
            .await?;
            return package_lifecycle_summary(operation);
        }
        let definition = package_definition(input.package_id)?;
        if definition.required && input.action == PackageLifecycleAction::Disable {
            return Err(package_lifecycle_error(
                input.package_id,
                PackageReasonCode::PermissionDenied,
                "必装功能包不可停用",
            ));
        }
        let current = a4n_db::package::host_package_state(
            self.database.connection(),
            input.package_id.as_str(),
        )
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("功能包状态不存在: {}", input.package_id.as_str()))
        })?;
        if input.action == PackageLifecycleAction::Enable
            && !matches!(
                installation_state(&current.installation_state),
                PackageInstallationState::Installed
            )
        {
            return Err(package_lifecycle_error(
                input.package_id,
                match installation_state(&current.installation_state) {
                    PackageInstallationState::Incompatible => PackageReasonCode::Incompatible,
                    _ => PackageReasonCode::NotInstalled,
                },
                "功能包未处于可启用状态",
            ));
        }
        if input.action == PackageLifecycleAction::Enable {
            self.ensure_dependencies_enabled(&definition).await?;
        }
        let expected_version = requested_version.unwrap_or(current.status_version);
        let operation = a4n_db::package::start_package_lifecycle(
            self.database.connection(),
            &input.operation_id,
            input.package_id.as_str(),
            action,
            input.requested_version,
            user_id,
            expected_version,
        )
        .await?;

        package_lifecycle_summary(operation)
    }

    /// 推进一个已持久化的包操作。停用只在所有现有 owner 都进入安全终态后
    /// 才关闭 Host 状态；外部 provider 仍有 job 时保持 waiting_external。
    pub(crate) async fn advance_package_lifecycle(
        &self,
        operation_id: &str,
    ) -> Result<a4n_db_models::models::package::host_package_operation::Model, AppError> {
        let operation =
            a4n_db::package::package_operation(self.database.connection(), operation_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))?;
        if matches!(
            operation.state.as_str(),
            a4n_db::package::OPERATION_COMPLETED | a4n_db::package::OPERATION_FAILED
        ) {
            return Ok(operation);
        }

        if operation.action == "enable" {
            if self
                .package_health(PackageId::parse(&operation.package_id).ok_or_else(|| {
                    AppError::InvalidState {
                        current: format!("未知功能包: {}", operation.package_id),
                        operation: "推进功能包启用操作".to_string(),
                    }
                })?)
                .await
                != PackageHealth::Healthy
            {
                return a4n_db::package::finish_package_lifecycle(
                    self.database.connection(),
                    &operation.operation_id,
                    operation.status_version,
                    a4n_db::package::OPERATION_FAILED,
                    a4n_db::package::HOST_STATE_DISABLED,
                    Some(PackageReasonCode::CapabilityUnavailable.as_str()),
                    Some("功能包依赖服务当前不可用"),
                )
                .await;
            }
            return a4n_db::package::finish_package_lifecycle(
                self.database.connection(),
                &operation.operation_id,
                operation.status_version,
                a4n_db::package::OPERATION_COMPLETED,
                a4n_db::package::HOST_STATE_ENABLED,
                None,
                None,
            )
            .await;
        }

        let package_id = operation.package_id.clone();
        let workload =
            a4n_db::package::package_workload_snapshot(self.database.connection(), &package_id)
                .await?;
        for task in workload
            .generation_tasks
            .iter()
            .filter(|task| task.status != "finalizing")
        {
            let outcome = self
                .request_generation_destructive_cancel(task)
                .await
                .map_err(|error| AppError::InvalidState {
                    current: format!("功能包 {package_id} generation 排空失败: {error}"),
                    operation: "推进功能包停用操作".to_string(),
                })?;
            if matches!(
                outcome,
                crate::generation::terminal::GenerationTerminalWriteOutcome::NoLiveSession
            ) {
                self.interrupt_generation_destructive_task(
                    task,
                    a4n_service_protocol::InterruptionReason::SessionOwnerLost,
                    "功能包停用时 generation 执行 owner 已丢失".to_string(),
                )
                .await
                .map_err(|error| AppError::InvalidState {
                    current: format!("功能包 {package_id} generation 恢复收尾失败: {error}"),
                    operation: "推进功能包停用操作".to_string(),
                })?;
            }
        }
        for task_id in &workload.queue_task_ids {
            self.task_queue.cancel_persisted_task(task_id).await?;
        }
        for run_id in &workload.agent_run_ids {
            self.agent.cancel_run_for_package_shutdown(*run_id).await?;
        }

        let remaining =
            a4n_db::package::package_workload_snapshot(self.database.connection(), &package_id)
                .await?;
        if remaining.is_empty() {
            return a4n_db::package::update_package_lifecycle_progress(
                self.database.connection(),
                &operation.operation_id,
                operation.status_version,
                a4n_db::package::OPERATION_DRAINING,
                None,
                Some("工作已排空，等待 managed service 安全停止"),
            )
            .await;
        }
        let next_state = if remaining.has_external_work() {
            a4n_db::package::OPERATION_WAITING_EXTERNAL
        } else {
            a4n_db::package::OPERATION_DRAINING
        };
        let reason = if remaining.unknown_task_ids.is_empty() {
            None
        } else {
            Some(PackageReasonCode::MaintenanceUnavailable.as_str())
        };
        a4n_db::package::update_package_lifecycle_progress(
            self.database.connection(),
            &operation.operation_id,
            operation.status_version,
            next_state,
            reason,
            Some("仍有在途工作，继续保持停用准入关闭"),
        )
        .await
    }

    /// 在 Runtime 完成共享 service 安全停止后，提交停用终态。
    ///
    /// 重新读取工作事实，避免把一次旧快照当成排空确认。
    pub(crate) async fn finalize_package_disable(
        &self,
        operation_id: &str,
    ) -> Result<a4n_db_models::models::package::host_package_operation::Model, AppError> {
        let operation =
            a4n_db::package::package_operation(self.database.connection(), operation_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))?;
        if matches!(
            operation.state.as_str(),
            a4n_db::package::OPERATION_COMPLETED | a4n_db::package::OPERATION_FAILED
        ) {
            return Ok(operation);
        }
        if operation.action != "disable" {
            return Err(AppError::InvalidState {
                current: format!("功能包操作 {} 不是停用操作", operation.operation_id),
                operation: "提交功能包停用终态".to_string(),
            });
        }
        let remaining = a4n_db::package::package_workload_snapshot(
            self.database.connection(),
            &operation.package_id,
        )
        .await?;
        if !remaining.is_empty() {
            return self.advance_package_lifecycle(operation_id).await;
        }
        a4n_db::package::finish_package_lifecycle(
            self.database.connection(),
            &operation.operation_id,
            operation.status_version,
            a4n_db::package::OPERATION_COMPLETED,
            a4n_db::package::HOST_STATE_DISABLED,
            None,
            None,
        )
        .await
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
            a4n_db::package::host_package_state(self.database.connection(), package_id.as_str())
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
    ) -> Result<Vec<a4n_db_models::models::package::host_package_operation::Model>, AppError> {
        a4n_db::package::pending_package_operations(self.database.connection()).await
    }

    pub(crate) async fn enabled_package_service_names(&self) -> Result<Vec<String>, AppError> {
        let states = a4n_db::package::list_host_package_states(self.database.connection()).await?;
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
        a4n_db::package::package_workload_snapshot(self.database.connection(), package_id).await
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
        let states = a4n_db::package::list_host_package_states(self.database.connection()).await?;
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
        let operation =
            a4n_db::package::package_operation(self.database.connection(), operation_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))?;
        package_lifecycle_summary(operation)
    }

    pub(crate) async fn package_preferences(
        &self,
        user_id: i64,
    ) -> Result<PackagePreferencesSummary, AppError> {
        let hidden_package_ids =
            a4n_db::package::package_preference(self.database.connection(), user_id)
                .await?
                .map(|preference| parse_hidden_package_ids(preference.hidden_package_ids))
                .transpose()?
                .unwrap_or_default();
        Ok(PackagePreferencesSummary {
            host_scope: a4n_api_contract::package::HOST_SCOPE.to_string(),
            hidden_package_ids,
        })
    }

    pub(crate) async fn update_package_preferences(
        &self,
        user_id: i64,
        input: UpdatePackagePreferencesInput,
    ) -> Result<PackagePreferencesSummary, AppError> {
        let catalog = package_catalog();
        let requested = input.hidden_package_ids.into_iter().collect::<HashSet<_>>();
        if requested.iter().any(|package| package.is_required()) {
            return Err(AppError::InvalidArgument("必装功能包不能隐藏".to_string()));
        }
        let hidden_package_ids = catalog
            .packages
            .into_iter()
            .map(|package| package.id)
            .filter(|package| requested.contains(package))
            .collect::<Vec<_>>();
        a4n_db::package::upsert_package_preference(
            self.database.connection(),
            user_id,
            serde_json::to_value(
                hidden_package_ids
                    .iter()
                    .map(|package| package.as_str())
                    .collect::<Vec<_>>(),
            )?,
        )
        .await?;
        self.package_preferences(user_id).await
    }

    async fn package_command_availability_for_descriptor(
        &self,
        user_id: i64,
        descriptor: &CommandDescriptor,
    ) -> Result<PackageCommandAvailability, AppError> {
        let state = a4n_db::package::host_package_state(
            self.database.connection(),
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
            let Some(state) = a4n_db::package::host_package_state(
                self.database.connection(),
                dependency.as_str(),
            )
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

#[async_trait::async_trait]
impl a4n_api_contract::package::PackageMutationApplicationPort for OrchestratorContext {
    async fn initialize_states(&self) -> Result<(), AppError> {
        self.initialize_package_states().await
    }

    async fn ensure_execution_enabled(&self, package_id: PackageId) -> Result<(), AppError> {
        self.ensure_package_execution_enabled(package_id).await
    }

    async fn start_lifecycle(
        &self,
        user_id: i64,
        input: StartPackageLifecycleInput,
    ) -> Result<PackageLifecycleSummary, AppError> {
        self.start_package_lifecycle(user_id, input).await
    }

    async fn advance_lifecycle(
        &self,
        operation_id: &str,
    ) -> Result<PackageLifecycleSummary, AppError> {
        package_lifecycle_summary(self.advance_package_lifecycle(operation_id).await?)
    }

    async fn finalize_disable(&self, operation_id: &str) -> Result<(), AppError> {
        self.finalize_package_disable(operation_id)
            .await
            .map(|_| ())
    }

    async fn update_preferences(
        &self,
        user_id: i64,
        input: a4n_api_contract::package::UpdatePackagePreferencesInput,
    ) -> Result<a4n_api_contract::package::PackagePreferencesSummary, AppError> {
        self.update_package_preferences(user_id, input).await
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
    operation: a4n_db_models::models::package::host_package_operation::Model,
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
            &database,
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

        let state = host_package_state(&database, PackageId::Agent.as_str())
            .await
            .expect("读取 Agent 功能包状态")
            .expect("Agent 功能包状态必须存在");
        start_package_lifecycle(
            &database,
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
            &database,
            &[
                PackageId::Agent.as_str().to_string(),
                PackageId::AiWorkflow.as_str().to_string(),
            ],
        )
        .await
        .expect("初始化 Agent 及其依赖功能包状态");

        let dependency = host_package_state(&database, PackageId::AiWorkflow.as_str())
            .await
            .expect("读取 AI workflow 功能包状态")
            .expect("AI workflow 功能包状态必须存在");
        start_package_lifecycle(
            &database,
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
