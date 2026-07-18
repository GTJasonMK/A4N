mod claim;
mod lease;
mod media;
mod worker;

#[cfg(test)]
use lease::cleanup_retry_after_seconds;
#[cfg(test)]
use media::cleanup_requires_rag;
pub(crate) use media::project_media_path_prefix;
pub use worker::{
    drain_project_cleanup_for, recover_project_cleanup_outbox, spawn_project_cleanup_worker,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_retry_backoff_is_capped() {
        assert_eq!(cleanup_retry_after_seconds(1), 5);
        assert_eq!(cleanup_retry_after_seconds(2), 10);
        assert_eq!(cleanup_retry_after_seconds(7), 300);
        assert_eq!(cleanup_retry_after_seconds(100), 300);
    }

    #[test]
    fn minimal_runtime_treats_absent_rag_backend_as_noop() {
        assert!(!cleanup_requires_rag(crate::state::RuntimeProfile::Minimal));
        assert!(cleanup_requires_rag(crate::state::RuntimeProfile::Full));
        assert!(cleanup_requires_rag(crate::state::RuntimeProfile::Sidecar));
    }

    #[test]
    fn project_media_prefix_has_directory_boundary() {
        assert_eq!(project_media_path_prefix(1), "projects/1/");
        assert!(!project_media_path_prefix(10).starts_with(&project_media_path_prefix(1)));
    }
}
