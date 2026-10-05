use super::config::ManagedServiceConfig;
use super::errors::ServiceManagerError;
use super::health::http_health;
use super::integrity::BinaryIntegrityVerifier;
use super::manager::configured_auth_token;
use a4n_service_protocol::{
    HealthStatus, SIDECAR_AUTH_HEADER, SIDECAR_LAST_ERROR_ENV, SIDECAR_RESTART_COUNT_ENV,
    SIDECAR_SHUTDOWN_PATH, redact_sensitive_message,
};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

const SHUTDOWN_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const SIDECAR_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedSidecarState {
    Starting,
    Running,
    Stopping,
    BackingOff,
    CircuitOpen,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct ManagedSidecarStatus {
    pub state: ManagedSidecarState,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
enum StopMode {
    Graceful,
    Force,
}

type StopReply = oneshot::Sender<Result<(), ServiceManagerError>>;

#[derive(Debug)]
enum SupervisorCommand {
    Stop {
        mode: StopMode,
        reply: Option<StopReply>,
    },
}

#[derive(Debug)]
pub struct ManagedSidecarProcess {
    config: ManagedServiceConfig,
    command: mpsc::UnboundedSender<SupervisorCommand>,
    status: watch::Receiver<ManagedSidecarStatus>,
    task: Option<JoinHandle<Result<(), ServiceManagerError>>>,
    stop_failure: Option<String>,
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
            state: ManagedSidecarState::Running,
            pid: child.id(),
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
            stop_failure: None,
        })
    }

    pub fn service_name(&self) -> &str {
        &self.config.name
    }

    pub(super) fn status_receiver(&self) -> watch::Receiver<ManagedSidecarStatus> {
        self.status.clone()
    }

    pub async fn stop(mut self) -> Result<(), ServiceManagerError> {
        self.stop_in_place().await
    }

    pub async fn force_stop(mut self) -> Result<(), ServiceManagerError> {
        self.stop_in_place_with(StopMode::Force).await
    }

    /// Stop without transferring ownership to the caller. Package lifecycle
    /// retries keep a failed stop attached to the owning collection.
    pub async fn stop_in_place(&mut self) -> Result<(), ServiceManagerError> {
        self.stop_in_place_with(StopMode::Graceful).await
    }

    async fn stop_in_place_with(&mut self, mode: StopMode) -> Result<(), ServiceManagerError> {
        if let Some(reason) = &self.stop_failure {
            return Err(ServiceManagerError::SupervisorTaskFailed {
                service: self.config.name.clone(),
                reason: format!("previous stop attempt failed: {reason}"),
            });
        }

        if self.task.as_ref().is_some_and(|task| task.is_finished()) {
            return self.take_task_result().await;
        }

        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .command
            .send(SupervisorCommand::Stop {
                mode,
                reply: Some(reply_tx),
            })
            .is_err()
        {
            return self.take_task_result().await;
        }

        match reply_rx.await {
            Ok(Ok(())) => self.take_task_result().await,
            Ok(Err(error)) => Err(error),
            Err(error) => Err(ServiceManagerError::SupervisorTaskFailed {
                service: self.config.name.clone(),
                reason: format!("stop reply channel failed: {error}"),
            }),
        }
    }

    async fn take_task_result(&mut self) -> Result<(), ServiceManagerError> {
        let Some(task) = self.task.take() else {
            return Ok(());
        };
        let result = match task.await {
            Ok(result) => result,
            Err(error) => Err(ServiceManagerError::SupervisorTaskFailed {
                service: self.config.name.clone(),
                reason: error.to_string(),
            }),
        };
        if let Err(error) = &result {
            self.stop_failure = Some(error.to_string());
        }
        result
    }
}

impl Drop for ManagedSidecarProcess {
    fn drop(&mut self) {
        if self.task.take().is_some() {
            let _ = self.command.send(SupervisorCommand::Stop {
                mode: StopMode::Force,
                reply: None,
            });
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
    enum StartupEvent {
        Healthy(Result<(), ServiceManagerError>),
        Command(Option<SupervisorCommand>),
    }

    let mut restart_count = 0;
    let mut stopping = false;
    loop {
        if stopping {
            let command = tokio::select! {
                command = commands.recv() => command,
                result = child.wait() => {
                    match result {
                        Ok(_) => {
                            set_status(&status, ManagedSidecarState::Stopped, None);
                            return Ok(());
                        }
                        Err(source) => {
                            return Err(ServiceManagerError::ProcessStatusFailed {
                                service: config.name.clone(),
                                source,
                            });
                        }
                    }
                }
            };
            let (mode, reply) = match command {
                Some(SupervisorCommand::Stop { mode, reply }) => (mode, reply),
                None => (StopMode::Force, None),
            };
            match stop_child(&config, &mut child, &client, mode).await {
                Ok(()) => {
                    if let Some(reply) = reply {
                        let _ = reply.send(Ok(()));
                    }
                    set_status(&status, ManagedSidecarState::Stopped, None);
                    return Ok(());
                }
                Err(error) => {
                    set_status(&status, ManagedSidecarState::Stopping, child.id());
                    if let Some(reply) = reply {
                        let _ = reply.send(Err(error));
                        continue;
                    }
                    return Err(error);
                }
            }
        }

        let exit_error = tokio::select! {
            command = commands.recv() => {
                let (mode, reply) = match command {
                    Some(SupervisorCommand::Stop { mode, reply }) => (mode, reply),
                    None => (StopMode::Force, None),
                };
                match stop_child(&config, &mut child, &client, mode).await {
                    Ok(()) => {
                        if let Some(reply) = reply {
                            let _ = reply.send(Ok(()));
                        }
                        set_status(&status, ManagedSidecarState::Stopped, None);
                        return Ok(());
                    }
                    Err(error) => {
                        stopping = true;
                        set_status(&status, ManagedSidecarState::Stopping, child.id());
                        if let Some(reply) = reply {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        return Err(error);
                    }
                }
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
                set_status(&status, ManagedSidecarState::CircuitOpen, None);
                return Err(ServiceManagerError::RestartLimitExceeded {
                    service: config.name.clone(),
                    restarts: restart_count,
                    last_error,
                });
            }

            restart_count += 1;
            let backoff_ms = config.restart_policy.backoff_ms(restart_count);
            set_status(&status, ManagedSidecarState::BackingOff, None);
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
                command = commands.recv() => {
                    if let Some(SupervisorCommand::Stop {
                        reply: Some(reply), ..
                    }) = command
                    {
                        let _ = reply.send(Ok(()));
                    }
                    set_status(&status, ManagedSidecarState::Stopped, None);
                    return Ok(());
                }
            }

            set_status(&status, ManagedSidecarState::Starting, None);
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
                result = wait_until_healthy(&config, &mut next, &client) => {
                    StartupEvent::Healthy(result)
                }
                command = commands.recv() => StartupEvent::Command(command),
            };
            match startup_result {
                StartupEvent::Healthy(Ok(())) => {
                    child = next;
                    set_status(&status, ManagedSidecarState::Running, child.id());
                    break;
                }
                StartupEvent::Healthy(Err(error)) => {
                    let next_last_error = status_error(&error);
                    next.kill().await.map_err(|kill_source| {
                        ServiceManagerError::StartupCleanupFailed {
                            service: config.name.clone(),
                            startup: Box::new(error),
                            kill_source,
                        }
                    })?;
                    last_error = next_last_error;
                }
                StartupEvent::Command(command) => {
                    let (mode, reply) = match command {
                        Some(SupervisorCommand::Stop { mode, reply }) => (mode, reply),
                        None => (StopMode::Force, None),
                    };
                    match stop_child(&config, &mut next, &client, mode).await {
                        Ok(()) => {
                            if let Some(reply) = reply {
                                let _ = reply.send(Ok(()));
                            }
                            set_status(&status, ManagedSidecarState::Stopped, None);
                            return Ok(());
                        }
                        Err(error) => {
                            if let Some(reply) = reply {
                                let _ = reply.send(Err(error));
                                child = next;
                                stopping = true;
                                set_status(&status, ManagedSidecarState::Stopping, child.id());
                                break;
                            }
                            return Err(error);
                        }
                    }
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
    command.env_clear();
    if binary
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.trim_end_matches(".exe") == "a4n-component-host")
    {
        // Keep old packages launchable while new catalog entries execute the
        // component binary directly.
        command.args(["--service", config.name.as_str()]);
    }
    command.envs(config.env.clone());
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", system_root);
    }
    command.env(SIDECAR_RESTART_COUNT_ENV, restart_count.to_string());
    if let Some(last_error) = last_error {
        command.env(SIDECAR_LAST_ERROR_ENV, last_error);
    }
    configure_parent_death_signal(&mut command);
    command.kill_on_drop(true);
    command
        .spawn()
        .map_err(|source| ServiceManagerError::SpawnFailed {
            service: config.name.clone(),
            source,
        })
}

#[cfg(target_os = "linux")]
fn configure_parent_death_signal(command: &mut Command) {
    // A forced Host/Control shutdown must not orphan its sidecars under PID 1.
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn prctl(option: i32, ...) -> i32;
            }
            const PR_SET_PDEATHSIG: i32 = 1;
            const SIGTERM: i32 = 15;
            if prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_parent_death_signal(_: &mut Command) {}

async fn stop_child(
    config: &ManagedServiceConfig,
    child: &mut Child,
    client: &reqwest::Client,
    mode: StopMode,
) -> Result<(), ServiceManagerError> {
    if matches!(mode, StopMode::Graceful) && request_graceful_shutdown(config, client).await? {
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

async fn request_graceful_shutdown(
    config: &ManagedServiceConfig,
    client: &reqwest::Client,
) -> Result<bool, ServiceManagerError> {
    let mut request = client.post(config.endpoint_url(SIDECAR_SHUTDOWN_PATH)?);
    if let Some(token) = configured_auth_token(config)? {
        request = request.header(SIDECAR_AUTH_HEADER, token);
    }
    Ok(
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
        },
    )
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
    state: ManagedSidecarState,
    pid: Option<u32>,
) {
    sender.send_replace(ManagedSidecarStatus { state, pid });
}

fn status_error(error: &ServiceManagerError) -> String {
    redact_sensitive_message(error.to_string())
        .chars()
        .take(512)
        .collect()
}
