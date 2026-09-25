use super::*;

#[derive(Clone)]
pub struct AgentRuntimeShared {
    pub(crate) agent_admission: Arc<Mutex<()>>,
    pub(crate) runs: Arc<RwLock<HashMap<i64, AgentRuntimeEntry>>>,
}

#[derive(Default)]
pub(crate) struct AgentRuntimeEntry {
    pub(crate) parent_run_id: Option<i64>,
    pub(crate) snapshot: Option<RuntimeConfigSnapshot>,
    pub(crate) cancellation: Option<CancellationToken>,
    pub(crate) finished: bool,
}

impl AgentRuntimeShared {
    pub fn new() -> Self {
        Self {
            agent_admission: Arc::new(Mutex::new(())),
            runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn lock_agent_admission(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.agent_admission.lock().await
    }
}

/// Keep lineage while descendants need it, even when an intermediate Run is
/// waiting without a task or a configuration snapshot in this process.
pub(super) fn prune_finished(runs: &mut HashMap<i64, AgentRuntimeEntry>) {
    loop {
        let parents = runs
            .values()
            .filter_map(|run| run.parent_run_id)
            .collect::<HashSet<_>>();
        let before = runs.len();
        runs.retain(|id, run| {
            !run.finished
                || run.cancellation.is_some()
                || run.snapshot.is_some()
                || parents.contains(id)
        });
        if runs.len() == before {
            break;
        }
    }
}

impl Default for AgentRuntimeShared {
    fn default() -> Self {
        Self::new()
    }
}
