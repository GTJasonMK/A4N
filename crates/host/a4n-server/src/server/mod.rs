mod worker;
use std::{
    env,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime},
};

use a4n_process_catalog::stateful_worker_catalog_entry_ref;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WorkerCapabilities, WorkerHealth,
    WorkerHealthStatus,
};

mod config;
#[cfg(feature = "test-embedded-processes")]
mod embedded;
mod health;
mod process;
mod startup;
mod types;

#[cfg(all(test, unix))]
mod tests;

use config::*;
use health::*;
use process::*;
use types::*;
use worker::catalog::*;
use worker::managed::*;

pub(crate) use startup::run;
