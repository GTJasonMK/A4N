use a4n_api_contract::package::PackageId;
use a4n_api_contract::package::{
    GetPackageOperationInput, ListPackageCommandAvailabilityInput, PackageCommandAvailability,
    PackageLifecycleAction, PackageLifecycleState, PackageLifecycleSummary, PackageOverview,
    PackagePreferencesSummary, StartPackageLifecycleInput, UpdatePackagePreferencesInput,
};
use a4n_error::AppError;
use std::time::Duration;

use crate::AppState;

pub async fn get_package_overview(state: &AppState) -> Result<Vec<PackageOverview>, AppError> {
    state.package_application().overview().await
}

pub async fn list_package_command_availability(
    state: &AppState,
    user_id: i64,
    input: ListPackageCommandAvailabilityInput,
) -> Result<Vec<PackageCommandAvailability>, AppError> {
    state
        .package_application()
        .list_command_availability(user_id, input)
        .await
}

pub async fn ensure_command_available(
    state: &AppState,
    user_id: i64,
    command: &str,
) -> Result<PackageCommandAvailability, AppError> {
    let availability = command_availability(state, user_id, command).await?;
    if availability.available {
        return Ok(availability);
    }
    Err(AppError::InvalidState {
        current: format!(
            "package_id={} reason_code={} status_version={}",
            availability.package_id.as_str(),
            availability.reason_code.as_str(),
            availability.status_version
        ),
        operation: format!("准入 Host command {}", availability.command),
    })
}

pub async fn command_availability(
    state: &AppState,
    user_id: i64,
    command: &str,
) -> Result<PackageCommandAvailability, AppError> {
    state
        .package_application()
        .command_availability(user_id, command)
        .await
}

pub async fn start_package_lifecycle(
    state: &AppState,
    user_id: i64,
    input: StartPackageLifecycleInput,
) -> Result<PackageLifecycleSummary, AppError> {
    let package_id = input.package_id;
    let action = input.action;
    let _agent_admission = if input.package_id == PackageId::Agent {
        Some(state.lock_agent_admission().await)
    } else {
        None
    };
    let summary = state
        .package_application()
        .start_lifecycle(
            user_id,
            StartPackageLifecycleInput {
                package_id,
                action,
                ..input
            },
        )
        .await?;
    if matches!(
        summary.state,
        PackageLifecycleState::Completed | PackageLifecycleState::Failed
    ) {
        return Ok(summary);
    }
    progress_package_lifecycle(state, &summary.operation_id, package_id, action).await?;
    state
        .package_application()
        .operation(
            user_id,
            GetPackageOperationInput {
                operation_id: summary.operation_id,
            },
        )
        .await
}

async fn start_package_services(state: &AppState, package_id: PackageId) -> Result<(), AppError> {
    let services = state.package_application().owned_services(package_id);
    state
        .start_managed_sidecars_for_services(&services)
        .await
        .map_err(|error| AppError::InvalidState {
            current: error,
            operation: format!("启动功能包 {} 的 managed service", package_id.as_str()),
        })
}

async fn progress_package_lifecycle(
    state: &AppState,
    operation_id: &str,
    package_id: PackageId,
    action: PackageLifecycleAction,
) -> Result<(), AppError> {
    if action == PackageLifecycleAction::Enable {
        start_package_services(state, package_id).await?;
    } else if state.package_application().has_workload(package_id).await? {
        // A disabling package is not included in the startup enabled-service
        // set. Keep its provider/worker service alive while durable workload
        // or cleanup/reconcile facts still require it, including after a
        // restart.
        start_package_services(state, package_id).await?;
    }

    let operation = state
        .package_application()
        .advance_lifecycle(operation_id)
        .await?;
    if action != PackageLifecycleAction::Disable
        || matches!(
            operation.state,
            PackageLifecycleState::Completed | PackageLifecycleState::Failed
        )
    {
        return Ok(());
    }
    if operation.state != PackageLifecycleState::Draining {
        return Ok(());
    }

    if state.package_application().has_workload(package_id).await? {
        return Ok(());
    }
    let services = state
        .package_application()
        .services_safe_to_stop(package_id)
        .await?;
    state
        .stop_managed_sidecars_for_services(&services)
        .await
        .map_err(|error| AppError::InvalidState {
            current: error,
            operation: format!("停止功能包 {} 的 managed service", package_id.as_str()),
        })?;
    state
        .package_application()
        .finalize_disable(operation_id)
        .await?;
    Ok(())
}

pub async fn get_package_operation(
    state: &AppState,
    user_id: i64,
    input: GetPackageOperationInput,
) -> Result<PackageLifecycleSummary, AppError> {
    state.package_application().operation(user_id, input).await
}

pub async fn get_package_preferences(
    state: &AppState,
    user_id: i64,
) -> Result<PackagePreferencesSummary, AppError> {
    state.package_application().preferences(user_id).await
}

pub async fn update_package_preferences(
    state: &AppState,
    user_id: i64,
    input: UpdatePackagePreferencesInput,
) -> Result<PackagePreferencesSummary, AppError> {
    state
        .package_application()
        .update_preferences(user_id, input)
        .await
}

/// 进程启动后持续接续未终态的 Host package operation。操作事实仍在 DB，
/// worker 只负责重新调用 orchestrator owner，不缓存第二份状态。
pub(crate) fn spawn_package_lifecycle_worker(state: AppState) {
    tokio::spawn(async move {
        loop {
            let operations = match state
                .package_application()
                .pending_lifecycle_operations()
                .await
            {
                Ok(operations) => operations,
                Err(error) => {
                    tracing::warn!(error = %error, "读取未完成功能包生命周期操作失败");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
            for operation in operations {
                let _agent_admission = if operation.package_id == PackageId::Agent.as_str() {
                    Some(state.lock_agent_admission().await)
                } else {
                    None
                };
                let Some(package_id) = PackageId::parse(&operation.package_id) else {
                    tracing::warn!(
                        operation_id = %operation.operation_id,
                        package_id = %operation.package_id,
                        "功能包操作包含未知 package id，保留生命周期操作"
                    );
                    continue;
                };
                let action = match operation.action.as_str() {
                    "enable" => PackageLifecycleAction::Enable,
                    "disable" => PackageLifecycleAction::Disable,
                    action => {
                        tracing::warn!(
                            operation_id = %operation.operation_id,
                            action,
                            "功能包操作包含未知 action，保留生命周期操作"
                        );
                        continue;
                    }
                };
                if let Err(error) =
                    progress_package_lifecycle(&state, &operation.operation_id, package_id, action)
                        .await
                {
                    tracing::warn!(
                        operation_id = %operation.operation_id,
                        error = %error,
                        "后台接续功能包生命周期操作失败，将保留非终态重试"
                    );
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

pub(crate) async fn package_execution_enabled(
    state: &AppState,
    package_id: PackageId,
) -> Result<bool, AppError> {
    state
        .package_application()
        .execution_enabled(package_id)
        .await
}

pub(crate) async fn ensure_package_execution_enabled(
    state: &AppState,
    package_id: PackageId,
) -> Result<(), AppError> {
    state
        .package_application()
        .ensure_execution_enabled(package_id)
        .await
}
