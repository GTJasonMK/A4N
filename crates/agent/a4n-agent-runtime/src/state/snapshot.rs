use super::*;

impl AgentRuntimeShared {
    pub async fn store_agent_runtime_snapshot(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
        value: RuntimeConfigSnapshot,
    ) {
        let mut runs = self.runs.write().await;
        let entry = runs.entry(run_id).or_default();
        entry.parent_run_id = parent_run_id;
        entry.snapshot = Some(value);
    }

    pub async fn agent_runtime_snapshot(&self, run_id: i64) -> Option<RuntimeConfigSnapshot> {
        self.runs
            .read()
            .await
            .get(&run_id)
            .and_then(|entry| entry.snapshot.clone())
    }

    pub(crate) async fn take_agent_runtime_snapshot(
        &self,
        run_id: i64,
    ) -> Option<RuntimeConfigSnapshot> {
        let mut runs = self.runs.write().await;
        let snapshot = runs.get_mut(&run_id).and_then(|entry| {
            entry.finished = true;
            entry.snapshot.take()
        });
        shared::prune_finished(&mut runs);
        snapshot
    }

    pub async fn remove_agent_runtime_snapshot_tree(&self, run_id: i64) {
        let mut runs = self.runs.write().await;
        let run_ids = runs
            .keys()
            .copied()
            .filter(|candidate| {
                is_descendant(*candidate, run_id, |candidate| {
                    runs.get(&candidate)
                        .and_then(|snapshot| snapshot.parent_run_id)
                })
            })
            .collect::<Vec<_>>();
        for run_id in run_ids {
            if let Some(entry) = runs.get_mut(&run_id) {
                entry.snapshot = None;
                entry.finished = true;
            }
        }
        shared::prune_finished(&mut runs);
    }
}
