use super::super::lease::run_with_lease_heartbeat;
use crate::AppState;
use std::collections::HashSet;

pub(super) async fn cleanup_candidate_paths(
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
        media_unreferenced_paths(state, paths),
    )
    .await?;
    if candidates.is_empty() {
        return Ok(candidates);
    }
    let candidates = run_with_lease_heartbeat(
        state,
        id,
        lease_token,
        media_unreferenced_paths(state, &candidates),
    )
    .await?;
    let protected = run_with_lease_heartbeat(
        state,
        id,
        lease_token,
        media_protected_paths(state, project_id, &candidates),
    )
    .await?;
    Ok(candidates
        .into_iter()
        .filter(|path| !protected.contains(path))
        .collect())
}

async fn media_unreferenced_paths(
    state: &AppState,
    paths: &[String],
) -> Result<Vec<String>, String> {
    if let Some(worker) = state.artifact_media_reference_application() {
        return worker
            .unreferenced_paths(a4n_artifact_media_contract::MediaPathsRequest {
                paths: paths.to_vec(),
            })
            .await;
    }
    state
        .image_media_reference_application()
        .unreferenced_media_paths(paths)
        .await
        .map_err(crate::actions::app_error_message)
}

async fn media_protected_paths(
    state: &AppState,
    project_id: i64,
    paths: &[String],
) -> Result<HashSet<String>, String> {
    if let Some(worker) = state.artifact_media_reference_application() {
        return worker
            .protected_paths(a4n_artifact_media_contract::ProjectPathsRequest {
                project_id,
                paths: paths.to_vec(),
            })
            .await
            .map(|paths| paths.into_iter().collect());
    }
    state
        .image_media_reference_application()
        .protected_image_publication_cleanup_paths(project_id, paths)
        .await
        .map_err(crate::actions::app_error_message)
}
