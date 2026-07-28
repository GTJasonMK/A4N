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
pub async fn with_llm_slot<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    LLM_SLOT_HELD.scope((), future).await
}

/// Marks the current async operation as already holding its image queue slot.
/// Nested provider calls can reuse that slot instead of waiting on themselves.
pub async fn with_image_slot<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    IMAGE_SLOT_HELD.scope((), future).await
}

/// Marks the current async operation as already holding its video queue slot.
/// Nested provider calls can reuse that slot instead of waiting on themselves.
pub async fn with_video_slot<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    VIDEO_SLOT_HELD.scope((), future).await
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
