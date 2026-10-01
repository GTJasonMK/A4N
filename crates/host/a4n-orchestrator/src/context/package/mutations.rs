use super::*;

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
