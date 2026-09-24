use super::*;

#[cfg(feature = "test-embedded-processes")]
pub(crate) const CONTROL_PROCESS_ENV: &str = "A4N_CONTROL_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const EDGE_PROCESS_ENV: &str = "A4N_EDGE_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const ADMIN_WORKER_PROCESS_ENV: &str = "A4N_ADMIN_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const ARTIFACT_MEDIA_WORKER_PROCESS_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const OUTLINE_WORKER_PROCESS_ENV: &str = "A4N_OUTLINE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const BLUEPRINT_WORKER_PROCESS_ENV: &str = "A4N_BLUEPRINT_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const ASSET_GRAPH_WORKER_PROCESS_ENV: &str = "A4N_ASSET_GRAPH_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const FORESHADOWING_WORKER_PROCESS_ENV: &str = "A4N_FORESHADOWING_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const OBJECT_PROFILE_WORKER_PROCESS_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const SCENE_PROFILE_WORKER_PROCESS_ENV: &str = "A4N_SCENE_PROFILE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const PROFILE_MATERIAL_WORKER_PROCESS_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const PROJECT_STATUS_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_STATUS_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const PORTRAIT_WORKER_PROCESS_ENV: &str = "A4N_PORTRAIT_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const SOURCE_SNAPSHOT_WORKER_PROCESS_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const THEME_READ_WORKER_PROCESS_ENV: &str = "A4N_THEME_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const PROJECT_READ_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const IMPORT_REVIEW_READ_WORKER_PROCESS_ENV: &str =
    "A4N_IMPORT_REVIEW_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const INSPIRATION_HISTORY_WORKER_PROCESS_ENV: &str =
    "A4N_INSPIRATION_HISTORY_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const PROJECT_ACCESS_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const PROJECT_COLLABORATION_WORKER_PROCESS_ENV: &str =
    "A4N_PROJECT_COLLABORATION_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const PROJECT_MUTATION_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const CHAPTER_SCENE_WORKER_PROCESS_ENV: &str = "A4N_CHAPTER_SCENE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const CHAPTER_CONTENT_WORKER_PROCESS_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const WORKFLOW_READ_WORKER_PROCESS_ENV: &str = "A4N_WORKFLOW_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const WORKFLOW_LIFECYCLE_WORKER_PROCESS_ENV: &str =
    "A4N_WORKFLOW_LIFECYCLE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const CONFIG_READ_WORKER_PROCESS_ENV: &str = "A4N_CONFIG_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const CHARACTER_MUTATION_WORKER_PROCESS_ENV: &str =
    "A4N_CHARACTER_MUTATION_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const VIDEO_RECORD_WORKER_PROCESS_ENV: &str = "A4N_VIDEO_RECORD_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const MANGA_RECORD_WORKER_PROCESS_ENV: &str = "A4N_MANGA_RECORD_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const CANVAS_RECORD_WORKER_PROCESS_ENV: &str = "A4N_CANVAS_RECORD_WORKER_PROCESS";

pub(crate) const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";
pub(crate) const EDGE_BINARY_ENV: &str = "A4N_EDGE_BINARY";
pub(crate) const AGENT_WORKER_CATALOG_NAME: &str = "agent-worker";
pub(crate) const RUNTIME_WORKER_RELOAD_MARKER_ENV: &str = "A4N_RUNTIME_WORKER_RELOAD_MARKER";
pub(crate) const CONTROL_READY_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const CONTROL_READY_RETRY_INTERVAL: Duration = Duration::from_millis(50);
pub(crate) const CONTROL_READY_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
pub(crate) const WORKER_READY_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const WORKER_READY_RETRY_INTERVAL: Duration = Duration::from_millis(50);
pub(crate) const WORKER_READY_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
pub(crate) const CHILD_MONITOR_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const WORKER_RESTART_DELAY: Duration = Duration::from_millis(250);

pub(crate) fn reload_marker_path() -> Option<PathBuf> {
    env::var_os(RUNTIME_WORKER_RELOAD_MARKER_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

pub(crate) fn worker_token_for_env(env_name: &str) -> anyhow::Result<String> {
    if let Some(value) = env::var_os(env_name) {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("{env_name} 必须是有效 UTF-8"))?;
        let value = value.trim().to_string();
        if value.is_empty() {
            anyhow::bail!("{env_name} 不能为空");
        }
        return Ok(value);
    }
    Ok(uuid::Uuid::new_v4().to_string())
}
