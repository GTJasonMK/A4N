use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) const LOCAL_STATE_SCHEMA_VERSION: u32 = 1;
pub(super) const DEFAULT_QDRANT_URL: &str = "http://127.0.0.1:6334";

#[derive(Clone, Copy)]
pub(crate) struct StartOptions {
    pub(crate) managed_sidecars: bool,
    pub(crate) qdrant: bool,
    pub(crate) full: bool,
}

pub(crate) struct DoctorOutcome {
    pub(crate) report: Value,
    pub(crate) healthy: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LocalState {
    pub(super) schema_version: u32,
    pub(super) server_pid: u32,
    pub(super) server_marker: String,
    pub(super) server_url: String,
    pub(super) server_log: PathBuf,
    pub(super) qdrant_pid: Option<u32>,
    pub(super) qdrant_marker: Option<String>,
    pub(super) qdrant_log: Option<PathBuf>,
}

pub(super) struct LocalPaths {
    pub(super) state: PathBuf,
    pub(super) server_log: PathBuf,
    pub(super) qdrant_log: PathBuf,
    pub(super) qdrant_root: PathBuf,
}

pub(super) struct LocalStateInspection {
    pub(super) server_process_alive: bool,
    pub(super) server_identity_matches: bool,
    pub(super) server_healthy: bool,
    pub(super) qdrant_owned: bool,
    pub(super) qdrant_process_alive: bool,
    pub(super) qdrant_identity_matches: bool,
}

impl LocalStateInspection {
    pub(super) fn running(&self) -> bool {
        self.server_identity_matches && self.server_healthy
    }

    pub(super) fn status(&self) -> &'static str {
        if self.server_process_alive && !self.server_identity_matches
            || self.qdrant_process_alive && !self.qdrant_identity_matches
        {
            "error"
        } else if !self.server_process_alive
            || !self.server_healthy
            || self.qdrant_owned && (!self.qdrant_process_alive || !self.qdrant_identity_matches)
        {
            "warning"
        } else {
            "ok"
        }
    }
}

impl LocalPaths {
    pub(super) fn new(config_path: &Path) -> Self {
        let root = config_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        Self {
            state: root.join("local.json"),
            server_log: root.join("local-server.log"),
            qdrant_log: root.join("local-qdrant.log"),
            qdrant_root: root.join("qdrant"),
        }
    }
}
