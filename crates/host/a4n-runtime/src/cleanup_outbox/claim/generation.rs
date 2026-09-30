use super::super::lease::renew_cleanup_lease;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_cleanup_contract::{GenerationCleanupOutcome, GenerationCleanupPayload};
use a4n_video::{VideoJobDeleteOutcome, VideoJobStatus, video_provider_capabilities};

pub(in crate::cleanup_outbox) async fn process_claimed_generation_cleanup(
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

    match &claim.payload {
        GenerationCleanupPayload::ProviderJob(payload) => {
            match process_claimed_provider_job_cleanup(state, &claim, payload).await {
                Ok(()) => Ok(()),
                Err(error) => retry_generation_cleanup_after_error(state, &claim, error).await,
            }
        }
        GenerationCleanupPayload::StagingArtifactDelete(payload) => {
            match crate::actions::video_generation::delete_remote_temp_media_artifact(
                &payload.artifact_id,
                &payload.sha256,
            )
            .await
            {
                Ok(crate::actions::video_generation::RemoteTempMediaDeleteOutcome::Deleted) => {
                    state
                        .cleanup_outbox_application()
                        .complete_claimed_generation_cleanup(
                            id,
                            lease_token,
                            GenerationCleanupOutcome::Deleted,
                        )
                        .await
                        .map_err(app_error_message)
                }
                Ok(crate::actions::video_generation::RemoteTempMediaDeleteOutcome::NotFound) => {
                    state
                        .cleanup_outbox_application()
                        .complete_claimed_generation_cleanup(
                            id,
                            lease_token,
                            GenerationCleanupOutcome::NotFound,
                        )
                        .await
                        .map_err(app_error_message)
                }
                Err(error) => retry_generation_cleanup_after_error(state, &claim, error).await,
            }
        }
        GenerationCleanupPayload::ArtifactUnbind(payload) => {
            match resolve_artifact_unbind_outcome(payload) {
                Ok(outcome) => state
                    .cleanup_outbox_application()
                    .complete_claimed_generation_cleanup(id, lease_token, outcome)
                    .await
                    .map_err(app_error_message),
                Err(error) => retry_generation_cleanup_after_error(state, &claim, error).await,
            }
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
                "generation cleanup 已按有界退避重试"
            );
        }
        a4n_cleanup_contract::GenerationCleanupRetryOutcome::Exhausted => {
            tracing::error!(
                cleanup_id = id,
                error,
                "generation cleanup 已显式标记 failed"
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
                    state.capabilities().video.clone(),
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
                    state.capabilities().video.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_cleanup_contract::GenerationArtifactUnbindPayload;

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
}
