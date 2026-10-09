use super::*;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BinaryFingerprint {
    pub(crate) modified: SystemTime,
    pub(crate) length: u64,
}

#[derive(Clone)]
pub(crate) struct WorkerSpec {
    pub(crate) metadata: &'static a4n_process_catalog::StatefulWorkerCatalogEntry,
    pub(crate) binary: PathBuf,
    pub(crate) address: SocketAddr,
    pub(crate) address_is_explicit: bool,
    pub(crate) control_addr: SocketAddr,
    pub(crate) token: String,
}

pub(crate) type ManagedWorkerConfig = WorkerSpec;

pub(crate) struct WorkerProcess {
    pub(crate) config: WorkerSpec,
    pub(crate) child: Child,
    pub(crate) fingerprint: BinaryFingerprint,
}

pub(crate) type ManagedWorker = WorkerProcess;

pub(crate) struct WorkerProcessRegistry {
    processes: Vec<WorkerProcess>,
    pub(crate) rollback_binaries: HashMap<PathBuf, tempfile::TempPath>,
}

impl WorkerProcessRegistry {
    #[cfg(test)]
    pub(crate) fn new(processes: Vec<WorkerProcess>) -> Self {
        Self {
            processes,
            rollback_binaries: HashMap::new(),
        }
    }

    pub(crate) fn with_rollback_binaries(
        processes: Vec<WorkerProcess>,
        rollback_binaries: HashMap<PathBuf, tempfile::TempPath>,
    ) -> Self {
        Self {
            processes,
            rollback_binaries,
        }
    }

    pub(crate) fn find_mut(&mut self, worker_name: &str) -> Option<&mut WorkerProcess> {
        self.processes
            .iter_mut()
            .find(|process| process.config.metadata.name == worker_name)
    }

    pub(crate) fn indices_for_binary(&self, binary: &Path) -> Vec<usize> {
        self.processes
            .iter()
            .enumerate()
            .filter_map(|(index, process)| (process.config.binary == binary).then_some(index))
            .collect()
    }
}

impl Deref for WorkerProcessRegistry {
    type Target = [WorkerProcess];

    fn deref(&self) -> &Self::Target {
        &self.processes
    }
}

impl DerefMut for WorkerProcessRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.processes
    }
}

#[derive(Debug)]
pub(crate) enum WorkerLifecycleEvent {
    Exited {
        worker_name: String,
        status: ExitStatus,
    },
    BinaryChanged {
        worker_name: String,
    },
}
