use super::super::*;

pub(crate) struct StartupConfig {
    pub(crate) web_addr: SocketAddr,
    pub(crate) control_addr: SocketAddr,
    pub(crate) workers: Vec<WorkerSpec>,
    pub(crate) control_binary: PathBuf,
    pub(crate) edge_binary: PathBuf,
    pub(crate) reload_marker: Option<PathBuf>,
    pub(crate) control_fingerprint: BinaryFingerprint,
    pub(crate) edge_fingerprint: BinaryFingerprint,
    pub(crate) reload_marker_fingerprint: Option<BinaryFingerprint>,
}

pub(crate) struct Runtime {
    pub(crate) config: StartupConfig,
    pub(crate) edge: Child,
    pub(crate) control: Child,
    pub(crate) workers: WorkerProcessRegistry,
    pub(crate) control_fingerprint: BinaryFingerprint,
    pub(crate) edge_fingerprint: BinaryFingerprint,
    pub(crate) reload_marker_fingerprint: Option<BinaryFingerprint>,
}
