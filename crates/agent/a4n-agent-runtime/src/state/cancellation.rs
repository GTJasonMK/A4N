use super::*;

impl AgentRuntimeShared {
    pub(crate) async fn register_agent_run_cancellation(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        let mut runs = self.runs.write().await;
        let entry = runs.entry(run_id).or_default();
        entry.parent_run_id = parent_run_id;
        entry.cancellation = Some(token.clone());
        entry.finished = false;
        token
    }

    pub async fn cancel_agent_run_tree(&self, run_id: i64) {
        let runs = self.runs.read().await;
        for (&active_run_id, entry) in runs.iter() {
            if is_descendant(active_run_id, run_id, |candidate| {
                runs.get(&candidate).and_then(|run| run.parent_run_id)
            }) && let Some(token) = &entry.cancellation
            {
                token.cancel();
            }
        }
    }

    pub(crate) async fn unregister_agent_run_cancellation(&self, run_id: i64) {
        let mut runs = self.runs.write().await;
        if let Some(entry) = runs.get_mut(&run_id) {
            entry.cancellation = None;
        }
        shared::prune_finished(&mut runs);
    }
}

pub(crate) fn is_descendant<F>(mut candidate: i64, ancestor: i64, mut parent_of: F) -> bool
where
    F: FnMut(i64) -> Option<i64>,
{
    let mut seen = HashSet::new();
    loop {
        if candidate == ancestor {
            return true;
        }
        if !seen.insert(candidate) {
            return false;
        }
        let Some(parent) = parent_of(candidate) else {
            return false;
        };
        candidate = parent;
    }
}
