use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) const RENDER_QUEUE_CAPACITY: usize = 4;

#[derive(Clone)]
pub(crate) struct RenderAdmission {
    slots: Arc<Semaphore>,
    active: Arc<Semaphore>,
}

impl RenderAdmission {
    pub(crate) fn new(max_concurrent: usize) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(
                max_concurrent.saturating_add(RENDER_QUEUE_CAPACITY),
            )),
            active: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub(crate) fn try_reserve_slot(&self) -> Option<OwnedSemaphorePermit> {
        self.slots.clone().try_acquire_owned().ok()
    }

    pub(crate) async fn acquire_active(&self) -> OwnedSemaphorePermit {
        self.active
            .clone()
            .acquire_owned()
            .await
            .expect("render admission semaphore must remain open")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RenderAdmission;

    #[tokio::test]
    async fn admission_bounds_slots_and_waits_for_an_active_permit() {
        let admission = RenderAdmission::new(2);
        let slots = (0..6)
            .map(|_| admission.try_reserve_slot())
            .collect::<Vec<_>>();
        assert!(slots.iter().all(Option::is_some));
        assert!(admission.try_reserve_slot().is_none());

        let first = admission.acquire_active().await;
        let second = admission.acquire_active().await;
        let mut waiting = tokio::spawn({
            let admission = admission.clone();
            async move { admission.acquire_active().await }
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut waiting)
                .await
                .is_err(),
            "a third active render must wait behind the configured limit"
        );
        drop(first);
        let third = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .expect("waiting render should be admitted after a permit is released")
            .expect("waiting render task should not panic");
        drop(second);
        drop(third);
    }
}
