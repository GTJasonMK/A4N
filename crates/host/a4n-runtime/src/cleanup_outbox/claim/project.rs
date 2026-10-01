use super::super::lease::{
    cleanup_retry_after_seconds, renew_cleanup_lease, run_blocking_with_lease_heartbeat,
    run_with_lease_heartbeat,
};
use super::super::media::{cleanup_requires_rag, project_media_path_prefix, remove_media_paths};
use super::paths::cleanup_candidate_paths;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_storage::paths::ProjectPaths;
use a4n_storage::{deletable_data_dir_path, remove_data_dir_in_data_dir};

pub(in crate::cleanup_outbox) async fn process_claimed_cleanup(
    state: &AppState,
    claim: a4n_cleanup_contract::ProjectDeletionCleanupClaim,
) -> Result<(), String> {
    let id = claim.record.id;
    let lease_token = claim
        .record
        .lease_token
        .as_deref()
        .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
    let project_id = claim.payload.project_id;
    let mut errors = Vec::new();

    renew_cleanup_lease(state, id, lease_token).await?;
    if cleanup_requires_rag(state.runtime_profile())
        && let Err(error) = run_with_lease_heartbeat(
            state,
            id,
            lease_token,
            state.rag_ingest().clear_project_rag(project_id),
        )
        .await
    {
        errors.push(format!("清理项目 RAG 失败: {error}"));
    }
    renew_cleanup_lease(state, id, lease_token).await?;

    match cleanup_candidate_paths(
        state,
        id,
        lease_token,
        project_id,
        &claim.payload.media_paths,
    )
    .await
    {
        Ok(paths) => {
            let data_dir = state.data_dir().to_path_buf();
            if let Err(error) =
                run_blocking_with_lease_heartbeat(state, id, lease_token, move || {
                    remove_media_paths(&data_dir, paths)
                })
                .await
            {
                errors.push(error);
            }
        }
        Err(error) => errors.push(format!("复核项目媒体路径引用失败: {error}")),
    }

    let prefix = project_media_path_prefix(project_id);
    match run_with_lease_heartbeat(
        state,
        id,
        lease_token,
        state.persisted_media_path_prefix_has_reference(&prefix),
    )
    .await
    {
        Ok(true) => errors.push(format!("项目目录仍有持久化媒体引用: {prefix}")),
        Ok(false) => {
            match run_with_lease_heartbeat(
                state,
                id,
                lease_token,
                media_project_has_active_publication(state, project_id),
            )
            .await
            {
                Ok(true) => errors.push(format!(
                    "项目仍有未收敛的图片发布意图，暂不删除目录: project_id={project_id}"
                )),
                Ok(false) => {
                    let data_dir = state.data_dir().to_path_buf();
                    let project_root = ProjectPaths::new(&data_dir, project_id)
                        .root()
                        .to_path_buf();
                    if let Err(error) =
                        run_blocking_with_lease_heartbeat(state, id, lease_token, move || {
                            deletable_data_dir_path(
                                &data_dir,
                                &project_root,
                                "项目删除 cleanup 数据目录",
                            )?;
                            remove_data_dir_in_data_dir(
                                &data_dir,
                                &project_root,
                                "项目删除 cleanup 数据目录",
                            )
                        })
                        .await
                    {
                        errors.push(error);
                    }
                }
                Err(error) => errors.push(format!("复核图片发布意图失败: {error}")),
            }
        }
        Err(error) => errors.push(format!("复核项目目录媒体引用失败: {error}")),
    }

    if errors.is_empty() {
        state
            .cleanup_outbox_application()
            .complete_claimed_cleanup(id, lease_token)
            .await
            .map_err(app_error_message)
    } else {
        let retry_after_seconds = cleanup_retry_after_seconds(claim.record.attempt_count);
        state
            .cleanup_outbox_application()
            .retry_claimed_cleanup(id, lease_token, errors.join("；"), retry_after_seconds)
            .await
            .map_err(app_error_message)
    }
}

pub(in crate::cleanup_outbox) async fn process_claimed_file_artifact_cleanup(
    state: &AppState,
    claim: a4n_cleanup_contract::FileArtifactCleanupClaim,
) -> Result<(), String> {
    let id = claim.record.id;
    let lease_token = claim
        .record
        .lease_token
        .as_deref()
        .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
    renew_cleanup_lease(state, id, lease_token).await?;
    let result = match cleanup_candidate_paths(
        state,
        id,
        lease_token,
        claim.payload.project_id,
        &claim.payload.paths,
    )
    .await
    {
        Ok(paths) => {
            let data_dir = state.data_dir().to_path_buf();
            run_blocking_with_lease_heartbeat(state, id, lease_token, move || {
                remove_media_paths(&data_dir, paths)
            })
            .await
        }
        Err(error) => Err(format!("复核文件 artifact 引用失败: {error}")),
    };
    match result {
        Ok(()) => state
            .cleanup_outbox_application()
            .complete_claimed_cleanup(id, lease_token)
            .await
            .map_err(app_error_message),
        Err(error) => {
            let retry_after_seconds = cleanup_retry_after_seconds(claim.record.attempt_count);
            state
                .cleanup_outbox_application()
                .retry_claimed_cleanup(id, lease_token, error, retry_after_seconds)
                .await
                .map_err(app_error_message)
        }
    }
}

async fn media_project_has_active_publication(
    state: &AppState,
    project_id: i64,
) -> Result<bool, String> {
    if let Some(worker) = state.artifact_media_reference_application() {
        return worker
            .project_has_publication(a4n_artifact_media_contract::ProjectPublicationRequest {
                project_id,
            })
            .await;
    }
    state
        .image_media_reference_application()
        .project_has_active_image_publication(project_id)
        .await
        .map_err(app_error_message)
}
