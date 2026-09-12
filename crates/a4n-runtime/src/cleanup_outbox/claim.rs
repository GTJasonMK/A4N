use super::lease::{
    cleanup_retry_after_seconds, renew_cleanup_lease, run_blocking_with_lease_heartbeat,
    run_with_lease_heartbeat,
};
use super::media::{cleanup_requires_rag, project_media_path_prefix, remove_media_paths};
use crate::AppState;
use crate::actions::app_error_message;
use a4n_cleanup_contract::{GenerationCleanupOutcome, GenerationCleanupPayload};
use a4n_storage::paths::ProjectPaths;
use a4n_storage::{deletable_data_dir_path, remove_data_dir_in_data_dir};
use a4n_video::{VideoJobDeleteOutcome, VideoJobStatus, video_provider_capabilities};

pub(super) async fn process_claimed_cleanup(
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
        state
            .image_media_reference_application()
            .media_path_prefix_has_persisted_reference(&prefix),
    )
    .await
    {
        Ok(true) => errors.push(format!("项目目录仍有持久化媒体引用: {prefix}")),
        Ok(false) => {
            match run_with_lease_heartbeat(
                state,
                id,
                lease_token,
                state
                    .image_media_reference_application()
                    .project_has_active_image_publication(project_id),
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

pub(super) async fn process_claimed_file_artifact_cleanup(
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

async fn cleanup_candidate_paths(
    state: &AppState,
    id: i64,
    lease_token: &str,
    project_id: i64,
    paths: &[String],
) -> Result<Vec<String>, String> {
    let candidates = run_with_lease_heartbeat(
        state,
        id,
        lease_token,
        state
            .image_media_reference_application()
            .unreferenced_media_paths(paths),
    )
    .await?;
    if candidates.is_empty() {
        return Ok(candidates);
    }
    let candidates = run_with_lease_heartbeat(
        state,
        id,
        lease_token,
        state
            .image_media_reference_application()
            .unreferenced_media_paths(&candidates),
    )
    .await?;
    let protected = run_with_lease_heartbeat(
        state,
        id,
        lease_token,
        state
            .image_media_reference_application()
            .protected_image_publication_cleanup_paths(project_id, &candidates),
    )
    .await?;
    Ok(candidates
        .into_iter()
        .filter(|path| !protected.contains(path))
        .collect())
}

pub(super) async fn process_claimed_media_workflow_reset(
    state: &AppState,
    claim: a4n_cleanup_contract::MediaWorkflowV2ResetClaim,
) -> Result<(), String> {
    let id = claim.record.id;
    let lease_token = claim
        .record
        .lease_token
        .as_deref()
        .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
    let project_id = claim.payload.project_id;
    renew_cleanup_lease(state, id, lease_token).await?;
    let result = state
        .cleanup_outbox_application()
        .reset_media_workflows_to_v2(project_id, id)
        .await
        .map_err(app_error_message);
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

pub(super) async fn process_claimed_generation_cleanup(
    state: &AppState,
    claim: a4n_cleanup_contract::GenerationCleanupClaim,
) -> Result<(), String> {
    let id = claim.record.id;
    let lease_token = claim
        .record
        .lease_token
        .as_deref()
        .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
    renew_cleanup_lease(state, id, lease_token).await?;
    if let GenerationCleanupPayload::ProviderJob(payload) = &claim.payload {
        return match process_claimed_provider_job_cleanup(state, &claim, payload).await {
            Ok(()) => Ok(()),
            Err(error) => retry_generation_cleanup_after_error(state, &claim, error).await,
        };
    }
    if let GenerationCleanupPayload::StagingArtifactDelete(payload) = &claim.payload {
        let result = crate::actions::video_generation::delete_remote_temp_media_artifact(
            &payload.artifact_id,
            &payload.sha256,
        )
        .await;
        return match result {
            Ok(crate::actions::video_generation::RemoteTempMediaDeleteOutcome::Deleted) => state
                .cleanup_outbox_application()
                .complete_claimed_generation_cleanup(
                    id,
                    lease_token,
                    GenerationCleanupOutcome::Deleted,
                )
                .await
                .map_err(app_error_message),
            Ok(crate::actions::video_generation::RemoteTempMediaDeleteOutcome::NotFound) => state
                .cleanup_outbox_application()
                .complete_claimed_generation_cleanup(
                    id,
                    lease_token,
                    GenerationCleanupOutcome::NotFound,
                )
                .await
                .map_err(app_error_message),
            Err(error) => retry_generation_cleanup_after_error(state, &claim, error).await,
        };
    }
    if let GenerationCleanupPayload::ArtifactUnbind(payload) = &claim.payload {
        // 临时 staging artifact 没有 durable 媒体表绑定；canonical Host owner 在此确认合同后
        // 只解除 generation 侧引用责任。物理删除由 generation_staging_artifact_delete 承担。
        return match resolve_artifact_unbind_outcome(payload) {
            Ok(outcome) => state
                .cleanup_outbox_application()
                .complete_claimed_generation_cleanup(id, lease_token, outcome)
                .await
                .map_err(app_error_message),
            Err(error) => retry_generation_cleanup_after_error(state, &claim, error).await,
        };
    }
    match generation_cleanup_resolution(&claim.payload) {
        Ok(outcome) => state
            .cleanup_outbox_application()
            .complete_claimed_generation_cleanup(id, lease_token, outcome)
            .await
            .map_err(app_error_message),
        Err(error) => {
            match state
                .cleanup_outbox_application()
                .retry_or_fail_claimed_generation_cleanup(id, lease_token, error.to_string())
                .await
                .map_err(app_error_message)?
            {
                a4n_cleanup_contract::GenerationCleanupRetryOutcome::Scheduled => {
                    tracing::warn!(
                        cleanup_id = id,
                        attempt_count = claim.record.attempt_count + 1,
                        error,
                        "generation cleanup 暂无可执行 owner，已按有界退避重试"
                    );
                }
                a4n_cleanup_contract::GenerationCleanupRetryOutcome::Exhausted => {
                    tracing::error!(
                        cleanup_id = id,
                        attempt_count = claim.record.attempt_count,
                        error,
                        "generation cleanup 重试耗尽，已显式标记 failed"
                    );
                }
            }
            Ok(())
        }
    }
}

fn resolve_artifact_unbind_outcome(
    payload: &a4n_cleanup_contract::GenerationArtifactUnbindPayload,
) -> Result<GenerationCleanupOutcome, String> {
    let artifact_id = payload.artifact_id.trim();
    let revision = payload.revision.trim();
    if artifact_id.is_empty() || revision.is_empty() {
        return Err("generation_artifact_unbind 缺少 artifact_id 或 revision".to_string());
    }
    // remote temp media 使用 32 位 hex artifact_id；其他 temporary 合同只要非空即允许解绑。
    if artifact_id.len() == 32 && !artifact_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "generation_artifact_unbind artifact_id 非法: {artifact_id}"
        ));
    }
    if revision.len() > 256 {
        return Err("generation_artifact_unbind revision 过长".to_string());
    }
    Ok(GenerationCleanupOutcome::Unbound)
}

async fn retry_generation_cleanup_after_error(
    state: &AppState,
    claim: &a4n_cleanup_contract::GenerationCleanupClaim,
    error: String,
) -> Result<(), String> {
    let id = claim.record.id;
    let lease_token = claim
        .record
        .lease_token
        .as_deref()
        .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
    match state
        .cleanup_outbox_application()
        .retry_or_fail_claimed_generation_cleanup(id, lease_token, error.clone())
        .await
        .map_err(app_error_message)?
    {
        a4n_cleanup_contract::GenerationCleanupRetryOutcome::Scheduled => {
            tracing::warn!(
                cleanup_id = id,
                error,
                "provider job cleanup 已按有界退避重试"
            );
        }
        a4n_cleanup_contract::GenerationCleanupRetryOutcome::Exhausted => {
            tracing::error!(
                cleanup_id = id,
                error,
                "provider job cleanup 已显式标记 failed"
            );
        }
    }
    Ok(())
}

enum ProviderCleanupAction {
    Complete(GenerationCleanupOutcome),
    Observe(i64),
    ScheduleDelete,
}

const PROVIDER_RECONCILE_INTERVAL_SECS: i64 = 60;

async fn process_claimed_provider_job_cleanup(
    state: &AppState,
    claim: &a4n_cleanup_contract::GenerationCleanupClaim,
    payload: &a4n_cleanup_contract::GenerationProviderJobCleanupPayload,
) -> Result<(), String> {
    let id = claim.record.id;
    let lease_token = claim
        .record
        .lease_token
        .as_deref()
        .ok_or_else(|| format!("cleanup outbox claim 缺少 lease token: id={id}"))?;
    let action = match payload.operation {
        a4n_cleanup_contract::GenerationProviderJobCleanupOperation::Cancel => {
            ProviderCleanupAction::Complete(GenerationCleanupOutcome::Unsupported)
        }
        a4n_cleanup_contract::GenerationProviderJobCleanupOperation::Delete => {
            let declared_capabilities = video_provider_capabilities(&payload.provider_kind)
                .ok_or_else(|| format!("未知视频生成后端: {}", payload.provider_kind))?;
            if !declared_capabilities.delete {
                ProviderCleanupAction::Complete(GenerationCleanupOutcome::Unsupported)
            } else {
                let config = state
                    .video_provider_config()
                    .for_provider_job(
                        claim.record.project_id,
                        &payload.provider_kind,
                        &payload.provider_job_id,
                    )
                    .await
                    .map_err(app_error_message)?;
                renew_cleanup_lease(state, id, lease_token).await?;
                match crate::sidecar_clients::delete_video_job(
                    state.runtime_profile(),
                    state.capabilities(),
                    &config,
                    &payload.provider_job_id,
                    claim.record.project_id,
                    None,
                )
                .await?
                {
                    VideoJobDeleteOutcome::Deleted => {
                        ProviderCleanupAction::Complete(GenerationCleanupOutcome::Deleted)
                    }
                    VideoJobDeleteOutcome::NotFound => {
                        ProviderCleanupAction::Complete(GenerationCleanupOutcome::NotFound)
                    }
                }
            }
        }
        a4n_cleanup_contract::GenerationProviderJobCleanupOperation::Reconcile => {
            let config = state
                .video_provider_config()
                .for_provider_job(
                    claim.record.project_id,
                    &payload.provider_kind,
                    &payload.provider_job_id,
                )
                .await
                .map_err(app_error_message)?;
            let capabilities = video_provider_capabilities(&config.backend)
                .ok_or_else(|| format!("未知视频生成后端: {}", config.backend))?;
            if !capabilities.status_query {
                ProviderCleanupAction::Complete(GenerationCleanupOutcome::Unsupported)
            } else {
                renew_cleanup_lease(state, id, lease_token).await?;
                let status = crate::sidecar_clients::video_job_status(
                    state.runtime_profile(),
                    state.capabilities(),
                    &config,
                    &payload.provider_job_id,
                    claim.record.project_id,
                    None,
                )
                .await?;
                match status {
                    VideoJobStatus::Pending | VideoJobStatus::Running => {
                        ProviderCleanupAction::Observe(PROVIDER_RECONCILE_INTERVAL_SECS)
                    }
                    VideoJobStatus::Failed(_) => {
                        ProviderCleanupAction::Complete(GenerationCleanupOutcome::RemoteFailed)
                    }
                    VideoJobStatus::Cancelled => {
                        ProviderCleanupAction::Complete(GenerationCleanupOutcome::Cancelled)
                    }
                    VideoJobStatus::NotFound => {
                        ProviderCleanupAction::Complete(GenerationCleanupOutcome::NotFound)
                    }
                    VideoJobStatus::Completed => ProviderCleanupAction::ScheduleDelete,
                }
            }
        }
    };

    match action {
        ProviderCleanupAction::Complete(outcome) => state
            .cleanup_outbox_application()
            .complete_claimed_generation_cleanup(id, lease_token, outcome)
            .await
            .map_err(app_error_message),
        ProviderCleanupAction::Observe(retry_after_seconds) => state
            .cleanup_outbox_application()
            .observe_claimed_generation_reconcile(id, lease_token, retry_after_seconds)
            .await
            .map(|outcome| {
                if matches!(
                    outcome,
                    a4n_cleanup_contract::GenerationCleanupObservationOutcome::DeadlineExceeded
                ) {
                    tracing::warn!(cleanup_id = id, "provider job reconcile 已超过 30 天期限");
                }
            })
            .map_err(app_error_message),
        ProviderCleanupAction::ScheduleDelete => state
            .cleanup_outbox_application()
            .schedule_provider_delete_from_reconcile(id, lease_token)
            .await
            .map(|_| ())
            .map_err(app_error_message),
    }
}

fn generation_cleanup_resolution(
    payload: &GenerationCleanupPayload,
) -> Result<GenerationCleanupOutcome, &'static str> {
    match payload {
        GenerationCleanupPayload::ProviderJob(_) => {
            Err("generation_provider_job_cleanup 必须由 provider adapter 处理")
        }
        GenerationCleanupPayload::StagingArtifactDelete(_) => {
            Err("generation_staging_artifact_delete 必须由异步 Host adapter 处理")
        }
        GenerationCleanupPayload::ArtifactUnbind(_) => {
            Err("generation_artifact_unbind 必须由 resolve_artifact_unbind_outcome 处理")
        }
    }
}

#[cfg(test)]
mod generation_cleanup_tests {
    use super::*;
    use a4n_cleanup_contract::{
        GenerationArtifactUnbindPayload, GenerationProviderJobCleanupOperation,
        GenerationProviderJobCleanupPayload, GenerationStagingArtifactDeletePayload,
    };

    #[test]
    fn generation_cleanup_never_substitutes_an_unavailable_operation() {
        let provider = GenerationCleanupPayload::ProviderJob(
            GenerationProviderJobCleanupPayload::new(
                GenerationProviderJobCleanupOperation::Cancel,
                "video".to_string(),
                "job-1".to_string(),
                "a".repeat(64),
            )
            .unwrap(),
        );
        assert!(generation_cleanup_resolution(&provider).is_err());

        let staging = GenerationCleanupPayload::StagingArtifactDelete(
            GenerationStagingArtifactDeletePayload {
                version: 1,
                artifact_id: "0123456789abcdef0123456789abcdef".to_string(),
                sha256: "a".repeat(64),
                revision: "a".repeat(64),
            },
        );
        assert!(generation_cleanup_resolution(&staging).is_err());
    }

    #[test]
    fn artifact_unbind_accepts_remote_temp_media_contract() {
        let payload = GenerationArtifactUnbindPayload {
            version: 1,
            artifact_id: "0123456789abcdef0123456789abcdef".to_string(),
            revision: "b".repeat(64),
        };
        assert_eq!(
            resolve_artifact_unbind_outcome(&payload).expect("unbind"),
            GenerationCleanupOutcome::Unbound
        );
    }

    #[test]
    fn artifact_unbind_rejects_invalid_hex_id_and_empty_revision() {
        let invalid_hex = GenerationArtifactUnbindPayload {
            version: 1,
            artifact_id: "0123456789abcdef0123456789abcdez".to_string(),
            revision: "rev".to_string(),
        };
        assert!(
            resolve_artifact_unbind_outcome(&invalid_hex)
                .expect_err("invalid hex")
                .contains("非法")
        );

        let empty = GenerationArtifactUnbindPayload {
            version: 1,
            artifact_id: "ok".to_string(),
            revision: "  ".to_string(),
        };
        assert!(
            resolve_artifact_unbind_outcome(&empty)
                .expect_err("empty revision")
                .contains("缺少")
        );
    }

    #[test]
    fn provider_cleanup_cannot_bypass_its_adapter() {
        let payload = GenerationCleanupPayload::ProviderJob(
            GenerationProviderJobCleanupPayload::new(
                GenerationProviderJobCleanupOperation::Delete,
                "cliproxy-openai-video".to_string(),
                "job-1".to_string(),
                "c".repeat(64),
            )
            .unwrap(),
        );
        assert!(generation_cleanup_resolution(&payload).is_err());
    }
}
