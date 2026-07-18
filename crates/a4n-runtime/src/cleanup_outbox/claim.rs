use super::lease::{
    cleanup_retry_after_seconds, renew_cleanup_lease, run_blocking_with_lease_heartbeat,
    run_with_lease_heartbeat,
};
use super::media::{cleanup_requires_rag, project_media_path_prefix, remove_media_paths};
use crate::AppState;
use crate::actions::app_error_message;
use crate::file_utils::{deletable_data_dir_path, remove_data_dir_in_data_dir};
use a4n_core::files::paths::ProjectPaths;

pub(super) async fn process_claimed_cleanup(
    state: &AppState,
    claim: a4n_db::cleanup_outbox::ClaimedProjectCleanup,
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
    if cleanup_requires_rag(state.runtime_profile) {
        if let Err(error) = run_with_lease_heartbeat(
            state,
            id,
            lease_token,
            state.orchestrator.clear_project_rag(project_id),
        )
        .await
        {
            errors.push(format!("清理项目 RAG 失败: {error}"));
        }
    }
    renew_cleanup_lease(state, id, lease_token).await?;

    match run_with_lease_heartbeat(
        state,
        id,
        lease_token,
        state
            .orchestrator
            .unreferenced_media_paths(&claim.payload.media_paths),
    )
    .await
    {
        Ok(paths) => {
            let data_dir = state.config.data_dir.clone();
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
        state
            .orchestrator
            .media_path_prefix_has_persisted_reference(&prefix),
    )
    .await
    {
        Ok(true) => errors.push(format!("项目目录仍有持久化媒体引用: {prefix}")),
        Ok(false) => {
            let data_dir = state.config.data_dir.clone();
            let project_root = ProjectPaths::new(&data_dir, project_id)
                .root()
                .to_path_buf();
            if let Err(error) =
                run_blocking_with_lease_heartbeat(state, id, lease_token, move || {
                    deletable_data_dir_path(&data_dir, &project_root, "项目删除 cleanup 数据目录")?;
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
        Err(error) => errors.push(format!("复核项目目录媒体引用失败: {error}")),
    }

    if errors.is_empty() {
        state
            .orchestrator
            .complete_claimed_cleanup(id, lease_token)
            .await
            .map_err(app_error_message)
    } else {
        let retry_after_seconds = cleanup_retry_after_seconds(claim.record.attempt_count);
        state
            .orchestrator
            .retry_claimed_cleanup(id, lease_token, errors.join("；"), retry_after_seconds)
            .await
            .map_err(app_error_message)
    }
}

pub(super) async fn process_claimed_media_workflow_reset(
    state: &AppState,
    claim: a4n_db::cleanup_outbox::ClaimedMediaWorkflowV2Reset,
) -> Result<(), String> {
    let id = claim.record.id;
    let lease_token = claim
        .record
        .lease_token
        .as_deref()
        .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
    let project_id = claim.payload.project_id;
    renew_cleanup_lease(state, id, lease_token).await?;
    let outcome = state
        .orchestrator
        .reset_media_workflows_to_v2(project_id)
        .await;
    let result = match outcome {
        Ok(outcome) => {
            let data_dir = state.config.data_dir.clone();
            run_blocking_with_lease_heartbeat(state, id, lease_token, move || {
                remove_media_paths(&data_dir, outcome.media_paths)
            })
            .await
        }
        Err(error) => Err(app_error_message(error)),
    };
    match result {
        Ok(()) => state
            .orchestrator
            .complete_claimed_cleanup(id, lease_token)
            .await
            .map_err(app_error_message),
        Err(error) => {
            let retry_after_seconds = cleanup_retry_after_seconds(claim.record.attempt_count);
            state
                .orchestrator
                .retry_claimed_cleanup(id, lease_token, error, retry_after_seconds)
                .await
                .map_err(app_error_message)
        }
    }
}
