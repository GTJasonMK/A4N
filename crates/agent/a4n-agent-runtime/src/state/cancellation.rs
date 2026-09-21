use super::*;

impl AgentRuntimeShared {
    pub(crate) async fn register_agent_run_cancellation(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
    ) -> CancellationToken {
        let token = CancellationToken::new();
        self.agent_run_cancellations.lock().await.insert(
            run_id,
            AgentRunCancellation {
                parent_run_id,
                token: token.clone(),
            },
        );
        token
    }

    pub async fn cancel_agent_run_tree(&self, run_id: i64) {
        let cancellations = self.agent_run_cancellations.lock().await;
        for (&active_run_id, cancellation) in cancellations.iter() {
            if is_descendant(active_run_id, run_id, |candidate| {
                cancellations
                    .get(&candidate)
                    .and_then(|run| run.parent_run_id)
            }) {
                cancellation.token.cancel();
            }
        }
    }

    pub(crate) async fn unregister_agent_run_cancellation(&self, run_id: i64) {
        self.agent_run_cancellations.lock().await.remove(&run_id);
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
