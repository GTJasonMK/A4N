use super::claim::process_claimed_cleanup;
use super::claim::process_claimed_media_workflow_reset;
use crate::AppState;
use crate::actions::app_error_message;
use std::time::Duration;

const STARTUP_DRAIN_LIMIT: usize = 100;
const WORKER_INTERVAL: Duration = Duration::from_secs(30);

pub async fn drain_project_cleanup_for(
    state: &AppState,
    project_id: Option<i64>,
    limit: usize,
) -> Result<usize, String> {
    let mut processed = 0;
    while processed < limit {
        if let Some(claim) = state
            .orchestrator
            .claim_media_workflow_v2_reset(project_id)
            .await
            .map_err(app_error_message)?
        {
            process_claimed_media_workflow_reset(state, claim).await?;
            processed += 1;
            continue;
        }
        let Some(claim) = state
            .orchestrator
            .claim_project_deletion_cleanup(project_id)
            .await
            .map_err(app_error_message)?
        else {
            break;
        };
        process_claimed_cleanup(state, claim).await?;
        processed += 1;
    }
    Ok(processed)
}

pub async fn recover_project_cleanup_outbox(state: &AppState) {
    if let Err(error) = drain_project_cleanup_for(state, None, STARTUP_DRAIN_LIMIT).await {
        tracing::warn!(error = %error, "启动恢复项目 cleanup outbox 失败");
    }
}

pub fn spawn_project_cleanup_worker(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(WORKER_INTERVAL);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = drain_project_cleanup_for(&state, None, STARTUP_DRAIN_LIMIT).await {
                tracing::warn!(error = %error, "后台恢复项目 cleanup outbox 失败");
            }
        }
    });
}
