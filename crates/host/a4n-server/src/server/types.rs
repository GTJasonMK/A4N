use super::*;
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
    pub(crate) token: String,
}

pub(crate) type ManagedWorkerConfig = WorkerSpec;

pub(crate) struct WorkerProcess {
    pub(crate) config: WorkerSpec,
    pub(crate) child: Child,
    pub(crate) fingerprint: BinaryFingerprint,
    pub(crate) rollback_binary: Option<PathBuf>,
}

pub(crate) type ManagedWorker = WorkerProcess;

pub(crate) struct WorkerProcessRegistry(Vec<WorkerProcess>);

impl WorkerProcessRegistry {
    pub(crate) fn new(processes: Vec<WorkerProcess>) -> Self {
        Self(processes)
    }

    pub(crate) fn find_mut(&mut self, worker_name: &str) -> Option<&mut WorkerProcess> {
        self.0
            .iter_mut()
            .find(|process| process.config.metadata.name == worker_name)
    }
}

impl Deref for WorkerProcessRegistry {
    type Target = [WorkerProcess];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WorkerProcessRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
