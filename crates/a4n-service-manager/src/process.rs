use super::config::ManagedServiceConfig;
use super::errors::ServiceManagerError;
use super::health::http_health;
use super::integrity::BinaryIntegrityVerifier;
use a4n_service_protocol::{
    HealthStatus, SIDECAR_AUTH_HEADER, SIDECAR_AUTH_TOKEN_ENV, SIDECAR_LAST_ERROR_ENV,
    SIDECAR_RESTART_COUNT_ENV, redact_sensitive_message,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

const SHUTDOWN_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const SIDECAR_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedSidecarState {
    Starting,
    Running,
    BackingOff,
    CircuitOpen,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedSidecarStatus {
    pub service: String,
    pub state: ManagedSidecarState,
    pub pid: Option<u32>,
    pub restart_count: u32,
    pub next_restart_in_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum SupervisorCommand {
    GracefulStop,
    ForceStop,
}

#[derive(Debug)]
pub struct ManagedSidecarProcess {
    config: ManagedServiceConfig,
    command: mpsc::UnboundedSender<SupervisorCommand>,
    status: watch::Receiver<ManagedSidecarStatus>,
    task: Option<JoinHandle<Result<(), ServiceManagerError>>>,
}

impl ManagedSidecarProcess {
    pub(super) async fn start(
        config: ManagedServiceConfig,
        client: reqwest::Client,
        integrity: Option<BinaryIntegrityVerifier>,
    ) -> Result<Self, ServiceManagerError> {
        let mut child = spawn_child(&config, 0, None, integrity.as_ref())?;
        if let Err(startup) = wait_until_healthy(&config, &mut child, &client).await {
            if let Err(kill_source) = child.kill().await {
                return Err(ServiceManagerError::StartupCleanupFailed {
                    service: config.name.clone(),
                    startup: Box::new(startup),
                    kill_source,
                });
            }
            return Err(startup);
        }

        let status = ManagedSidecarStatus {
            service: config.name.clone(),
            state: ManagedSidecarState::Running,
            pid: child.id(),
            restart_count: 0,
            next_restart_in_ms: None,
            last_error: None,
        };
        let (status_tx, status_rx) = watch::channel(status);
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let supervisor_config = config.clone();
        let task = tokio::spawn(supervise(
            supervisor_config,
            client,
            integrity,
            child,
            command_rx,
            status_tx,
        ));
        Ok(Self {
            config,
            command: command_tx,
            status: status_rx,
            task: Some(task),
        })
    }

    pub fn service_name(&self) -> &str {
        &self.config.name
    }

    pub fn id(&self) -> Option<u32> {
        self.status.borrow().pid
    }

    pub fn status(&self) -> ManagedSidecarStatus {
        self.status.borrow().clone()
    }

    pub(super) fn status_receiver(&self) -> watch::Receiver<ManagedSidecarStatus> {
        self.status.clone()
    }

    pub async fn stop(mut self) -> Result<(), ServiceManagerError> {
        self.finish(SupervisorCommand::GracefulStop).await
    }

    pub async fn force_stop(mut self) -> Result<(), ServiceManagerError> {
        self.finish(SupervisorCommand::ForceStop).await
    }

    async fn finish(&mut self, command: SupervisorCommand) -> Result<(), ServiceManagerError> {
        let _ = self.command.send(command);
        let Some(task) = self.task.take() else {
            return Ok(());
        };
        task.await
            .map_err(|error| ServiceManagerError::SupervisorTaskFailed {
                service: self.config.name.clone(),
                reason: error.to_string(),
            })?
    }
}

impl Drop for ManagedSidecarProcess {
    fn drop(&mut self) {
        if self.task.take().is_some() {
            let _ = self.command.send(SupervisorCommand::ForceStop);
        }
    }
}

async fn supervise(
    config: ManagedServiceConfig,
    client: reqwest::Client,
    integrity: Option<BinaryIntegrityVerifier>,
    mut child: Child,
    mut commands: mpsc::UnboundedReceiver<SupervisorCommand>,
    status: watch::Sender<ManagedSidecarStatus>,
) -> Result<(), ServiceManagerError> {
    let mut restart_count = 0;
    loop {
        let exit_error = tokio::select! {
            command = commands.recv() => {
                stop_child(&config, &mut child, command.unwrap_or(SupervisorCommand::ForceStop)).await?;
                set_status(&status, &config.name, ManagedSidecarState::Stopped, None, restart_count, None, None);
                return Ok(());
            }
            result = child.wait() => {
                match result {
                    Ok(exit) => format!("process exited unexpectedly with status {:?}", exit.code()),
                    Err(error) => status_error(&ServiceManagerError::ProcessStatusFailed {
                        service: config.name.clone(),
                        source: error,
                    }),
                }
            }
        };

        let mut last_error = exit_error;
        loop {
            if restart_count >= config.restart_policy.max_restarts {
                set_status(
                    &status,
                    &config.name,
                    ManagedSidecarState::CircuitOpen,
                    None,
                    restart_count,
                    None,
                    Some(last_error.clone()),
                );
                return Err(ServiceManagerError::RestartLimitExceeded {
                    service: config.name.clone(),
                    restarts: restart_count,
                    last_error,
                });
            }

            restart_count += 1;
            let backoff_ms = config.restart_policy.backoff_ms(restart_count);
            set_status(
                &status,
                &config.name,
                ManagedSidecarState::BackingOff,
                None,
                restart_count,
                Some(backoff_ms),
                Some(last_error.clone()),
            );
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
                command = commands.recv() => {
                    let _ = command;
                    set_status(&status, &config.name, ManagedSidecarState::Stopped, None, restart_count, None, Some(last_error));
                    return Ok(());
                }
            }

            set_status(
                &status,
                &config.name,
                ManagedSidecarState::Starting,
                None,
                restart_count,
                None,
                Some(last_error.clone()),
            );
            let mut next = match spawn_child(
                &config,
                restart_count,
                Some(&last_error),
                integrity.as_ref(),
            ) {
                Ok(child) => child,
                Err(error) => {
                    last_error = status_error(&error);
                    continue;
                }
            };

            let startup_result = tokio::select! {
                result = wait_until_healthy(&config, &mut next, &client) => Ok(result),
                command = commands.recv() => Err(command.unwrap_or(SupervisorCommand::ForceStop)),
            };
            match startup_result {
                Err(command) => {
                    stop_child(&config, &mut next, command).await?;
                    set_status(
                        &status,
                        &config.name,
                        ManagedSidecarState::Stopped,
                        None,
                        restart_count,
                        None,
                        Some(last_error),
                    );
                    return Ok(());
                }
                Ok(Ok(())) => {
                    child = next;
                    set_status(
                        &status,
                        &config.name,
                        ManagedSidecarState::Running,
                        child.id(),
                        restart_count,
                        None,
                        Some(last_error),
                    );
                    break;
                }
                Ok(Err(error)) => {
                    let _ = next.kill().await;
                    last_error = status_error(&error);
                }
            }
        }
    }
}

fn spawn_child(
    config: &ManagedServiceConfig,
    restart_count: u32,
    last_error: Option<&str>,
    integrity: Option<&BinaryIntegrityVerifier>,
) -> Result<Child, ServiceManagerError> {
    let binary = config
        .binary
        .as_ref()
        .expect("validated sidecar binary must exist");
    if let Some(integrity) = integrity {
        integrity.verify(&config.name, binary)?;
    }
    let mut command = Command::new(binary);
    command.args(&config.args);
    command.env_clear();
    command.envs(config.env.clone());
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", system_root);
    }
    command.env(SIDECAR_RESTART_COUNT_ENV, restart_count.to_string());
    if let Some(last_error) = last_error {
        command.env(SIDECAR_LAST_ERROR_ENV, last_error);
    }
    command.kill_on_drop(true);
    command
        .spawn()
        .map_err(|source| ServiceManagerError::SpawnFailed {
            service: config.name.clone(),
            source,
        })
}

async fn stop_child(
    config: &ManagedServiceConfig,
    child: &mut Child,
    command: SupervisorCommand,
) -> Result<(), ServiceManagerError> {
    if matches!(command, SupervisorCommand::GracefulStop) && request_graceful_shutdown(config).await
    {
        match tokio::time::timeout(SIDECAR_DRAIN_TIMEOUT, child.wait()).await {
            Ok(Ok(_)) => return Ok(()),
            Ok(Err(source)) => {
                return Err(ServiceManagerError::ProcessStatusFailed {
                    service: config.name.clone(),
                    source,
                });
            }
            Err(_) => tracing::warn!(
                service = %config.name,
                timeout_ms = SIDECAR_DRAIN_TIMEOUT.as_millis(),
                "sidecar graceful shutdown timed out; forcing termination"
            ),
        }
    }
    child
        .kill()
        .await
        .map_err(|source| ServiceManagerError::StopFailed {
            service: config.name.clone(),
            source,
        })
}

async fn request_graceful_shutdown(config: &ManagedServiceConfig) -> bool {
    let Some(base_url) = config.url.as_deref() else {
        return false;
    };
    let mut request =
        reqwest::Client::new().post(format!("{}/shutdown", base_url.trim_end_matches('/')));
    if let Some(token) = config.env.get(SIDECAR_AUTH_TOKEN_ENV) {
        request = request.header(SIDECAR_AUTH_HEADER, token);
    }
    match tokio::time::timeout(SHUTDOWN_REQUEST_TIMEOUT, request.send()).await {
        Ok(Ok(response)) if response.status().is_success() => true,
        Ok(Ok(response)) => {
            tracing::warn!(
                service = %config.name,
                status = %response.status(),
                "sidecar graceful shutdown was rejected; forcing termination"
            );
            false
        }
        Ok(Err(error)) => {
            tracing::warn!(
                service = %config.name,
                error = %redact_sensitive_message(error.to_string()),
                "sidecar graceful shutdown request failed; forcing termination"
            );
            false
        }
        Err(_) => {
            tracing::warn!(
                service = %config.name,
                timeout_ms = SHUTDOWN_REQUEST_TIMEOUT.as_millis(),
                "sidecar graceful shutdown request timed out; forcing termination"
            );
            false
        }
    }
}

async fn wait_until_healthy(
    config: &ManagedServiceConfig,
    child: &mut Child,
    client: &reqwest::Client,
) -> Result<(), ServiceManagerError> {
    let deadline = Duration::from_millis(config.startup_timeout_ms);
    let start = std::time::Instant::now();
    let mut last_error = None;

    while start.elapsed() < deadline {
        if let Some(status) =
            child
                .try_wait()
                .map_err(|source| ServiceManagerError::ProcessStatusFailed {
                    service: config.name.clone(),
                    source,
                })?
        {
            return Err(ServiceManagerError::Exited {
                service: config.name.clone(),
                status: status.code(),
            });
        }
        match http_health(client, config, None).await {
            Ok(health) if health.status == HealthStatus::Healthy => return Ok(()),
            Ok(health) => {
                last_error = Some(ServiceManagerError::Unhealthy {
                    service: config.name.clone(),
                    status: health.status,
                });
            }
            Err(error) => last_error = Some(error),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(last_error.unwrap_or_else(|| ServiceManagerError::Timeout {
        service: config.name.clone(),
        timeout_ms: config.startup_timeout_ms,
    }))
}

fn set_status(
    sender: &watch::Sender<ManagedSidecarStatus>,
    service: &str,
    state: ManagedSidecarState,
    pid: Option<u32>,
    restart_count: u32,
    next_restart_in_ms: Option<u64>,
    last_error: Option<String>,
) {
    sender.send_replace(ManagedSidecarStatus {
        service: service.to_string(),
        state,
        pid,
        restart_count,
        next_restart_in_ms,
        last_error,
    });
}

fn status_error(error: &ServiceManagerError) -> String {
    redact_sensitive_message(error.to_string())
        .chars()
        .take(512)
        .collect()
}
