use super::*;

#[derive(Clone)]
pub struct AgentRuntimeShared {
    pub(crate) agent_admission: Arc<Mutex<()>>,
    pub(crate) agent_run_cancellations: Arc<Mutex<HashMap<i64, AgentRunCancellation>>>,
    pub(crate) agent_runtime_snapshots: Arc<RwLock<HashMap<i64, AgentRuntimeSnapshot>>>,
}

#[derive(Clone)]
pub(crate) struct AgentRunCancellation {
    pub(crate) parent_run_id: Option<i64>,
    pub(crate) token: CancellationToken,
}

#[derive(Clone)]
pub(crate) struct AgentRuntimeSnapshot {
    pub(crate) parent_run_id: Option<i64>,
    pub(crate) value: RuntimeConfigSnapshot,
}

impl AgentRuntimeShared {
    pub fn new() -> Self {
        Self {
            agent_admission: Arc::new(Mutex::new(())),
            agent_run_cancellations: Arc::new(Mutex::new(HashMap::new())),
            agent_runtime_snapshots: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn lock_agent_admission(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.agent_admission.lock().await
    }
}

impl Default for AgentRuntimeShared {
    fn default() -> Self {
        Self::new()
    }
}
