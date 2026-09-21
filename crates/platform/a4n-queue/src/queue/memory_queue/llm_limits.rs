use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Weak},
};

use a4n_domain::task_queue::LlmConcurrencyLimit;
use a4n_error::AppError;
use parking_lot::Mutex;
use tokio::sync::Notify;

use crate::priority::TaskPriority;

#[derive(Default)]
pub(super) struct LlmLimits(Mutex<HashMap<String, Weak<Gate>>>);

#[derive(Default)]
struct Gate {
    state: Mutex<GateState>,
    changed: Notify,
}

#[derive(Default)]
struct GateState {
    limit: usize,
    running: usize,
    ticket: u64,
    waiting: BTreeSet<(u8, u64)>,
}

pub(super) struct LlmPermit {
    gate: Arc<Gate>,
    ticket: (u8, u64),
    running: bool,
}

impl LlmLimits {
    pub(super) fn waiting_by_priority(&self) -> [usize; 3] {
        let mut counts = [0; 3];
        for gate in self.0.lock().values().filter_map(Weak::upgrade) {
            for &(rank, _) in &gate.state.lock().waiting {
                counts[usize::from(rank)] += 1;
            }
        }
        counts
    }

    pub(super) async fn acquire(
        &self,
        limit: &LlmConcurrencyLimit,
        priority: TaskPriority,
    ) -> Result<LlmPermit, AppError> {
        if limit.key.trim().is_empty() || limit.max_concurrent == 0 {
            return Err(AppError::InvalidArgument(
                "LLM 并发配置必须有标识且上限大于 0".into(),
            ));
        }
        let gate = {
            let mut gates = self.0.lock();
            // ponytail: scan active configuration keys; shard only if configuration churn becomes material.
            gates.retain(|_, gate| gate.strong_count() > 0);
            let gate = gates
                .get(&limit.key)
                .and_then(Weak::upgrade)
                .unwrap_or_default();
            gates.insert(limit.key.clone(), Arc::downgrade(&gate));
            gate
        };
        let ticket = {
            let mut state = gate.state.lock();
            state.limit = limit.max_concurrent;
            state.ticket = state
                .ticket
                .checked_add(1)
                .ok_or_else(|| AppError::InvalidArgument("LLM 排队序号溢出".into()))?;
            let rank = match priority {
                TaskPriority::High => 0,
                TaskPriority::Medium => 1,
                TaskPriority::Low => 2,
            };
            let ticket = (rank, state.ticket);
            state.waiting.insert(ticket);
            ticket
        };
        gate.changed.notify_waiters();
        let mut permit = LlmPermit {
            gate: gate.clone(),
            ticket,
            running: false,
        };
        loop {
            let changed = gate.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let mut state = gate.state.lock();
                if state.running < state.limit && state.waiting.first() == Some(&ticket) {
                    state.waiting.remove(&ticket);
                    state.running += 1;
                    permit.running = true;
                    gate.changed.notify_waiters();
                    return Ok(permit);
                }
            }
            changed.await;
        }
    }
}

impl Drop for LlmPermit {
    fn drop(&mut self) {
        let mut state = self.gate.state.lock();
        if self.running {
            state.running -= 1;
        } else {
            state.waiting.remove(&self.ticket);
        }
        drop(state);
        self.gate.changed.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn limit(key: &str, max_concurrent: usize) -> LlmConcurrencyLimit {
        LlmConcurrencyLimit {
            key: key.into(),
            max_concurrent,
        }
    }

    #[tokio::test]
    async fn configurations_are_independent_and_limits_change_without_replacing_active_gates() {
        let limits = LlmLimits::default();
        let first = limits
            .acquire(&limit("a", 1), TaskPriority::Medium)
            .await
            .unwrap();
        let other = limits
            .acquire(&limit("b", 1), TaskPriority::Medium)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_millis(10),
                limits.acquire(&limit("a", 1), TaskPriority::Medium)
            )
            .await
            .is_err()
        );
        let second = limits
            .acquire(&limit("a", 2), TaskPriority::Medium)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_millis(10),
                limits.acquire(&limit("a", 1), TaskPriority::Medium)
            )
            .await
            .is_err()
        );
        drop(first);
        assert!(
            tokio::time::timeout(
                Duration::from_millis(10),
                limits.acquire(&limit("a", 1), TaskPriority::High)
            )
            .await
            .is_err()
        );
        drop(second);
        let next = limits
            .acquire(&limit("a", 1), TaskPriority::High)
            .await
            .unwrap();
        drop((next, other));
        assert!(
            limits
                .acquire(&limit("invalid", 0), TaskPriority::Medium)
                .await
                .is_err()
        );
    }
}
