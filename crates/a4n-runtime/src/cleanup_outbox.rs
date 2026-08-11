mod claim;
mod lease;
mod media;
mod worker;

use std::collections::BTreeMap;

use crate::AppState;

#[cfg(test)]
use lease::cleanup_retry_after_seconds;
#[cfg(test)]
use media::cleanup_requires_rag;
pub(crate) use media::project_media_path_prefix;
pub use worker::{drain_cleanup_outbox, recover_cleanup_outbox, spawn_cleanup_worker};

fn project_id_from_file_artifact_path(path: &str) -> Result<i64, String> {
    if path.contains('\\') {
        return Err(format!("cleanup 文件路径不能包含 Windows 分隔符: {path}"));
    }
    let mut parts = path.split('/');
    if parts.next() != Some("projects") {
        return Err(format!("cleanup 文件路径不在项目目录: {path}"));
    }
    parts
        .next()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|project_id| *project_id > 0)
        .ok_or_else(|| format!("cleanup 文件路径项目 ID 无效: {path}"))
}

pub(crate) async fn enqueue_file_artifact_cleanup_by_project(
    state: &AppState,
    cleanup_token: &str,
    paths: &[String],
) -> Result<(), String> {
    let mut by_project = BTreeMap::<i64, Vec<String>>::new();
    for path in paths {
        by_project
            .entry(project_id_from_file_artifact_path(path)?)
            .or_default()
            .push(path.clone());
    }
    for (project_id, paths) in by_project {
        state
            .orchestrator()
            .insert_file_artifact_cleanup_intent(
                project_id,
                format!("file-artifact:{cleanup_token}:{project_id}"),
                paths,
            )
            .await
            .map_err(|error| format!("写入文件 cleanup intent 失败：{error}"))?;
    }
    Ok(())
}

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

    #[test]
    fn file_artifact_cleanup_path_requires_project_scope() {
        assert_eq!(
            project_id_from_file_artifact_path("projects/42/media/image.png"),
            Ok(42)
        );
        assert!(project_id_from_file_artifact_path("../image.png").is_err());
        assert!(project_id_from_file_artifact_path("projects/42\\media\\image.png").is_err());
        assert!(project_id_from_file_artifact_path("projects/42/..\\43/image.png").is_err());
        assert!(project_id_from_file_artifact_path("projects/0/image.png").is_err());
    }
}
