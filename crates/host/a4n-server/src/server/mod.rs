use std::{
    env,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime},
};

use a4n_process_catalog::stateful_worker_catalog_entry_ref;
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    AGENT_WORKER_AUTH_HEADER, WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WorkerCapabilities,
    WorkerHealth, WorkerHealthStatus,
};

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

mod config;
#[cfg(feature = "test-embedded-processes")]
mod embedded;
mod health;
mod process;
mod startup;
mod types;
mod worker_catalog;
mod worker_poll;
mod worker_restarts;
mod worker_supervision;

#[cfg(all(test, unix))]
mod tests;

use config::*;
use health::*;
use process::*;
use types::*;
use worker_catalog::*;
use worker_poll::*;
use worker_restarts::*;
use worker_supervision::*;

pub(crate) use startup::run;
