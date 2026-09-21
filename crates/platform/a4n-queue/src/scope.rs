use std::future::Future;

tokio::task_local! {
    static LLM_SLOT_HELD: ();
}

tokio::task_local! {
    static IMAGE_SLOT_HELD: ();
}

tokio::task_local! {
    static VIDEO_SLOT_HELD: ();
}

/// Marks the current async operation as already holding its LLM queue slot.
/// Nested provider calls can reuse that slot instead of waiting on themselves.
pub fn with_llm_slot<F, T>(future: F) -> impl Future<Output = T>
where
    F: Future<Output = T>,
{
    LLM_SLOT_HELD.scope((), Box::pin(future))
}

/// Marks the current async operation as already holding its image queue slot.
/// Nested provider calls can reuse that slot instead of waiting on themselves.
pub fn with_image_slot<F, T>(future: F) -> impl Future<Output = T>
where
    F: Future<Output = T>,
{
    IMAGE_SLOT_HELD.scope((), Box::pin(future))
}

/// Marks the current async operation as already holding its video queue slot.
/// Nested provider calls can reuse that slot instead of waiting on themselves.
pub fn with_video_slot<F, T>(future: F) -> impl Future<Output = T>
where
    F: Future<Output = T>,
{
    VIDEO_SLOT_HELD.scope((), Box::pin(future))
}

pub(crate) fn llm_slot_is_held() -> bool {
    LLM_SLOT_HELD.try_with(|_| true).unwrap_or(false)
}

pub(crate) fn image_slot_is_held() -> bool {
    IMAGE_SLOT_HELD.try_with(|_| true).unwrap_or(false)
}

pub(crate) fn video_slot_is_held() -> bool {
    VIDEO_SLOT_HELD.try_with(|_| true).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{llm_slot_is_held, with_llm_slot};

    #[tokio::test]
    async fn llm_scope_boxes_large_future_and_sets_marker() {
        let payload = [0_u8; 32 * 1024];
        let scoped = with_llm_slot(async move {
            std::hint::black_box(payload);
            llm_slot_is_held()
        });

        assert!(std::mem::size_of_val(&scoped) < 1024);
        assert!(scoped.await);
        assert!(!llm_slot_is_held());
    }
}
