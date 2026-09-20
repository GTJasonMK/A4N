use super::*;

impl AgentRuntimeShared {
    pub async fn store_agent_runtime_snapshot(
        &self,
        run_id: i64,
        parent_run_id: Option<i64>,
        value: RuntimeConfigSnapshot,
    ) {
        self.agent_runtime_snapshots.write().await.insert(
            run_id,
            AgentRuntimeSnapshot {
                parent_run_id,
                value,
            },
        );
    }

    pub async fn agent_runtime_snapshot(&self, run_id: i64) -> Option<RuntimeConfigSnapshot> {
        self.agent_runtime_snapshots
            .read()
            .await
            .get(&run_id)
            .map(|snapshot| snapshot.value.clone())
    }

    pub(crate) async fn take_agent_runtime_snapshot(
        &self,
        run_id: i64,
    ) -> Option<RuntimeConfigSnapshot> {
        self.agent_runtime_snapshots
            .write()
            .await
            .remove(&run_id)
            .map(|snapshot| snapshot.value)
    }

    pub async fn remove_agent_runtime_snapshot_tree(&self, run_id: i64) {
        let mut snapshots = self.agent_runtime_snapshots.write().await;
        let run_ids = snapshots
            .keys()
            .copied()
            .filter(|candidate| {
                is_descendant(*candidate, run_id, |candidate| {
                    snapshots
                        .get(&candidate)
                        .and_then(|snapshot| snapshot.parent_run_id)
                })
            })
            .collect::<Vec<_>>();
        for run_id in run_ids {
            snapshots.remove(&run_id);
        }
    }
}
