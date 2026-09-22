use super::claim::{
    process_claimed_cleanup, process_claimed_file_artifact_cleanup,
    process_claimed_generation_cleanup,
};
use super::publication::recover_image_publications;
use crate::actions::app_error_message;
use crate::{AppState, state::RuntimeProfile};
use std::time::Duration;

const STARTUP_DRAIN_LIMIT: usize = 100;
const CLEANUP_AUDIT_TTL_LIMIT: usize = 100;
const GENERATION_SNAPSHOT_TTL_LIMIT: usize = 100;
const GENERATION_TASK_TREE_TTL_LIMIT: usize = 32;
const WORKER_INTERVAL: Duration = Duration::from_secs(30);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CleanupKind {
    FileArtifact,
    ProjectDeletion,
    Generation,
}

const CLEANUP_KINDS: [CleanupKind; 3] = [
    CleanupKind::FileArtifact,
    CleanupKind::ProjectDeletion,
    CleanupKind::Generation,
];
const HOST_CLEANUP_KINDS: [CleanupKind; 2] =
    [CleanupKind::ProjectDeletion, CleanupKind::Generation];

fn cleanup_kinds_for(artifact_media_worker_enabled: bool) -> &'static [CleanupKind] {
    if artifact_media_worker_enabled {
        &HOST_CLEANUP_KINDS
    } else {
        &CLEANUP_KINDS
    }
}

fn artifact_media_worker_owns_file_artifacts(runtime_profile: RuntimeProfile) -> bool {
    matches!(runtime_profile, RuntimeProfile::Sidecar)
}

fn next_cleanup_kind(index: &mut usize, artifact_media_worker_enabled: bool) -> CleanupKind {
    let kinds = cleanup_kinds_for(artifact_media_worker_enabled);
    let kind = kinds[*index];
    *index = (*index + 1) % kinds.len();
    kind
}

pub async fn drain_cleanup_outbox(
    state: &AppState,
    project_id: Option<i64>,
    limit: usize,
) -> Result<usize, String> {
    let mut processed = 0;
    let mut first_error = None;
    let mut next_kind = 0;
    let mut empty_kinds = 0;
    let artifact_media_worker_enabled =
        artifact_media_worker_owns_file_artifacts(state.runtime_profile());
    let kinds = cleanup_kinds_for(artifact_media_worker_enabled);
    while processed < limit && empty_kinds < kinds.len() {
        let kind = next_cleanup_kind(&mut next_kind, artifact_media_worker_enabled);
        let mut claimed = false;
        match kind {
            CleanupKind::FileArtifact => {
                if let Some(claim) = state
                    .cleanup_outbox_application()
                    .claim_file_artifact_cleanup(project_id)
                    .await
                    .map_err(app_error_message)?
                {
                    claimed = true;
                    let id = claim.record.id;
                    if let Err(error) = process_claimed_file_artifact_cleanup(state, claim).await {
                        tracing::warn!(cleanup_id = id, error = %error, "文件 artifact cleanup 处理失败，继续 drain 其他任务");
                        first_error
                            .get_or_insert(format!("file_artifact_cleanup id={id}: {error}"));
                    }
                }
            }
            CleanupKind::ProjectDeletion => {
                if let Some(claim) = state
                    .cleanup_outbox_application()
                    .claim_project_deletion_cleanup(project_id)
                    .await
                    .map_err(app_error_message)?
                {
                    claimed = true;
                    let id = claim.record.id;
                    if let Err(error) = process_claimed_cleanup(state, claim).await {
                        tracing::warn!(cleanup_id = id, error = %error, "项目删除 cleanup 处理失败，继续 drain 其他任务");
                        first_error.get_or_insert(format!("project_delete id={id}: {error}"));
                    }
                }
            }
            CleanupKind::Generation => {
                if let Some(claim) = state
                    .cleanup_outbox_application()
                    .claim_generation_cleanup(project_id)
                    .await
                    .map_err(app_error_message)?
                {
                    claimed = true;
                    let id = claim.record.id;
                    if let Err(error) = process_claimed_generation_cleanup(state, claim).await {
                        tracing::warn!(cleanup_id = id, error = %error, "generation cleanup 处理失败，继续 drain 其他任务");
                        first_error.get_or_insert(format!("generation cleanup id={id}: {error}"));
                    }
                }
            }
        }
        if claimed {
            processed += 1;
            empty_kinds = 0;
        } else {
            empty_kinds += 1;
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(processed),
    }
}

pub async fn recover_cleanup_outbox(state: &AppState) {
    if let Err(error) = drain_cleanup_outbox(state, None, STARTUP_DRAIN_LIMIT).await {
        tracing::warn!(error = %error, "启动恢复 cleanup outbox 失败");
    }
    recover_image_publications(
        state,
        &mut None,
        artifact_media_worker_owns_file_artifacts(state.runtime_profile()),
    )
    .await;
    expire_generation_snapshot_bodies(state).await;
    purge_expired_generation_task_trees(state).await;
    purge_completed_cleanup_audits(state).await;
}

pub fn spawn_cleanup_worker(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(WORKER_INTERVAL);
        let mut published_after = None;
        let artifact_media_worker_enabled =
            artifact_media_worker_owns_file_artifacts(state.runtime_profile());
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = drain_cleanup_outbox(&state, None, STARTUP_DRAIN_LIMIT).await {
                tracing::warn!(error = %error, "后台恢复 cleanup outbox 失败");
            }
            recover_image_publications(&state, &mut published_after, artifact_media_worker_enabled)
                .await;
            expire_generation_snapshot_bodies(&state).await;
            purge_expired_generation_task_trees(&state).await;
            purge_completed_cleanup_audits(&state).await;
        }
    });
}

async fn expire_generation_snapshot_bodies(state: &AppState) {
    match state
        .generation_cleanup_application()
        .expire_generation_snapshot_bodies(GENERATION_SNAPSHOT_TTL_LIMIT)
        .await
    {
        Ok(expired) if expired > 0 => {
            tracing::info!(expired, "generation terminal snapshot 正文已按 TTL 清理")
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(error = %error, "generation terminal snapshot TTL 清理失败"),
    }
}

async fn purge_expired_generation_task_trees(state: &AppState) {
    match state
        .generation_cleanup_application()
        .purge_expired_generation_task_trees(GENERATION_TASK_TREE_TTL_LIMIT)
        .await
    {
        Ok(deleted) if deleted > 0 => {
            tracing::info!(deleted, "generation task tree 已按审计保留期清理")
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(error = %error, "generation task tree TTL 清理失败"),
    }
}

async fn purge_completed_cleanup_audits(state: &AppState) {
    match state
        .cleanup_outbox_application()
        .purge_completed_cleanup_audits(CLEANUP_AUDIT_TTL_LIMIT)
        .await
    {
        Ok(deleted) if deleted > 0 => {
            tracing::info!(deleted, "cleanup outbox 已完成审计记录已按 TTL 清理")
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(error = %error, "cleanup outbox 审计 TTL 清理失败"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CleanupKind, artifact_media_worker_owns_file_artifacts, cleanup_kinds_for,
        next_cleanup_kind,
    };
    use crate::state::RuntimeProfile;

    #[test]
    fn cleanup_kinds_rotate_before_repeating() {
        let mut index = 0;
        let order = (0..cleanup_kinds_for(false).len() * 2)
            .map(|_| next_cleanup_kind(&mut index, false))
            .collect::<Vec<_>>();
        assert_eq!(
            order,
            vec![
                CleanupKind::FileArtifact,
                CleanupKind::ProjectDeletion,
                CleanupKind::Generation,
                CleanupKind::FileArtifact,
                CleanupKind::ProjectDeletion,
                CleanupKind::Generation,
            ]
        );
    }

    #[test]
    fn host_cleanup_kind_set_excludes_artifact_owner_work() {
        let kinds = cleanup_kinds_for(true);
        assert_eq!(kinds.len(), 2);
        assert!(!kinds.contains(&CleanupKind::FileArtifact));
        assert_eq!(cleanup_kinds_for(false).len(), 3);
    }

    #[test]
    fn artifact_owner_is_selected_by_runtime_profile() {
        assert!(!artifact_media_worker_owns_file_artifacts(
            RuntimeProfile::Minimal
        ));
        assert!(!artifact_media_worker_owns_file_artifacts(
            RuntimeProfile::Full
        ));
        assert!(artifact_media_worker_owns_file_artifacts(
            RuntimeProfile::Sidecar
        ));
    }
}
