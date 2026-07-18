use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use a4n_service_manager::{ManagedServiceConfig, ServiceMode};

use super::contracts::ManagedServicePidStatus;

pub(super) struct ServicePidObservation {
    pub(super) pid: Option<u32>,
    pub(super) status: Option<ManagedServicePidStatus>,
    pub(super) source: Option<String>,
}

pub(super) fn service_pid_observation(
    config: &ManagedServiceConfig,
    service_pid_dir: Option<&Path>,
    managed_pid: Option<u32>,
) -> ServicePidObservation {
    if !matches!(config.mode, ServiceMode::Remote | ServiceMode::Sidecar) {
        return ServicePidObservation::none();
    }
    if let Some(pid) = managed_pid {
        return process_pid_observation(pid, "manager_owned");
    }
    let Some(service_pid_dir) = service_pid_dir else {
        return ServicePidObservation::none();
    };

    let source = Some("pid_file".to_string());
    let path = service_pid_dir.join(format!("{}.pid", config.name));
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return ServicePidObservation {
                pid: None,
                status: Some(ManagedServicePidStatus::Missing),
                source,
            };
        }
        Err(_) => {
            return ServicePidObservation {
                pid: None,
                status: Some(ManagedServicePidStatus::Unreadable),
                source,
            };
        }
    };

    let Ok(pid) = content.trim().parse::<u32>() else {
        return ServicePidObservation {
            pid: None,
            status: Some(ManagedServicePidStatus::Invalid),
            source,
        };
    };
    process_pid_observation_with_source(pid, source)
}

fn process_pid_observation(pid: u32, source: &str) -> ServicePidObservation {
    process_pid_observation_with_source(pid, Some(source.to_string()))
}

fn process_pid_observation_with_source(pid: u32, source: Option<String>) -> ServicePidObservation {
    if pid == 0 {
        return ServicePidObservation {
            pid: None,
            status: Some(ManagedServicePidStatus::Invalid),
            source,
        };
    }
    match probe_process(pid) {
        ProcessProbe::Alive => ServicePidObservation {
            pid: Some(pid),
            status: Some(ManagedServicePidStatus::Observed),
            source,
        },
        ProcessProbe::NotRunning => ServicePidObservation {
            pid: None,
            status: Some(ManagedServicePidStatus::Stale),
            source,
        },
        #[cfg(not(unix))]
        ProcessProbe::Unknown => ServicePidObservation {
            pid: None,
            status: Some(ManagedServicePidStatus::Unverifiable),
            source,
        },
    }
}

impl ServicePidObservation {
    fn none() -> Self {
        Self {
            pid: None,
            status: None,
            source: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessProbe {
    Alive,
    NotRunning,
    #[cfg(not(unix))]
    Unknown,
}

#[cfg(unix)]
fn probe_process(pid: u32) -> ProcessProbe {
    if pid > u32::try_from(i32::MAX).expect("i32::MAX always fits u32") {
        return ProcessProbe::NotRunning;
    }

    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }

    let pid = i32::try_from(pid).expect("pid was checked against i32::MAX");
    let result = unsafe { kill(pid, 0) };
    if result == 0 {
        ProcessProbe::Alive
    } else {
        ProcessProbe::NotRunning
    }
}

#[cfg(windows)]
fn probe_process(pid: u32) -> ProcessProbe {
    let output = std::process::Command::new("tasklist")
        .arg("/FI")
        .arg(format!("PID eq {pid}"))
        .arg("/FO")
        .arg("CSV")
        .arg("/NH")
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains(&pid.to_string()) {
                ProcessProbe::Alive
            } else {
                ProcessProbe::NotRunning
            }
        }
        Ok(_) => ProcessProbe::NotRunning,
        Err(_) => ProcessProbe::Unknown,
    }
}

#[cfg(not(any(unix, windows)))]
fn probe_process(_pid: u32) -> ProcessProbe {
    ProcessProbe::Unknown
}
