use super::*;

#[cfg(feature = "test-embedded-processes")]
pub(crate) const CONTROL_PROCESS_ENV: &str = "A4N_CONTROL_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const EDGE_PROCESS_ENV: &str = "A4N_EDGE_PROCESS";
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
pub(crate) const CONFIG_READ_WORKER_PROCESS_ENV: &str = "A4N_CONFIG_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const CHARACTER_MUTATION_WORKER_PROCESS_ENV: &str =
    "A4N_CHARACTER_MUTATION_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const VIDEO_RECORD_WORKER_PROCESS_ENV: &str = "A4N_VIDEO_RECORD_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
pub(crate) const MANGA_RECORD_WORKER_PROCESS_ENV: &str = "A4N_MANGA_RECORD_WORKER_PROCESS";
pub(crate) const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";
pub(crate) const EDGE_BINARY_ENV: &str = "A4N_EDGE_BINARY";
pub(crate) const AGENT_WORKER_BINARY_ENV: &str = "A4N_AGENT_WORKER_BINARY";
pub(crate) const AGENT_WORKER_CATALOG_NAME: &str = "agent-worker";
pub(crate) const NOVEL_PLANNING_WORKER_CATALOG_NAME: &str = "novel-planning-worker";
pub(crate) const NOVEL_PLANNING_WORKER_BINARY_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_BINARY";
pub(crate) const NOVEL_PLANNING_WORKER_TOKEN_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_TOKEN";
pub(crate) const NOVEL_PLANNING_WORKER_URL_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_URL";
pub(crate) const ARTIFACT_MEDIA_WORKER_CATALOG_NAME: &str = "artifact-media-worker";
pub(crate) const ARTIFACT_MEDIA_WORKER_BINARY_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_BINARY";
pub(crate) const ARTIFACT_MEDIA_WORKER_TOKEN_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_TOKEN";
pub(crate) const ARTIFACT_MEDIA_WORKER_URL_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_URL";
pub(crate) const OUTLINE_WORKER_CATALOG_NAME: &str = "outline-worker";
pub(crate) const OUTLINE_WORKER_BINARY_ENV: &str = "A4N_OUTLINE_WORKER_BINARY";
pub(crate) const OUTLINE_WORKER_TOKEN_ENV: &str = "A4N_OUTLINE_WORKER_TOKEN";
pub(crate) const OUTLINE_WORKER_URL_ENV: &str = "A4N_OUTLINE_WORKER_URL";
pub(crate) const BLUEPRINT_WORKER_CATALOG_NAME: &str = "blueprint-worker";
pub(crate) const BLUEPRINT_WORKER_BINARY_ENV: &str = "A4N_BLUEPRINT_WORKER_BINARY";
pub(crate) const BLUEPRINT_WORKER_TOKEN_ENV: &str = "A4N_BLUEPRINT_WORKER_TOKEN";
pub(crate) const BLUEPRINT_WORKER_URL_ENV: &str = "A4N_BLUEPRINT_WORKER_URL";
pub(crate) const ASSET_GRAPH_WORKER_CATALOG_NAME: &str = "asset-graph-worker";
pub(crate) const ASSET_GRAPH_WORKER_BINARY_ENV: &str = "A4N_ASSET_GRAPH_WORKER_BINARY";
pub(crate) const ASSET_GRAPH_WORKER_TOKEN_ENV: &str = "A4N_ASSET_GRAPH_WORKER_TOKEN";
pub(crate) const ASSET_GRAPH_WORKER_URL_ENV: &str = "A4N_ASSET_GRAPH_WORKER_URL";
pub(crate) const FORESHADOWING_WORKER_CATALOG_NAME: &str = "foreshadowing-worker";
pub(crate) const FORESHADOWING_WORKER_BINARY_ENV: &str = "A4N_FORESHADOWING_WORKER_BINARY";
pub(crate) const FORESHADOWING_WORKER_TOKEN_ENV: &str = "A4N_FORESHADOWING_WORKER_TOKEN";
pub(crate) const FORESHADOWING_WORKER_URL_ENV: &str = "A4N_FORESHADOWING_WORKER_URL";
pub(crate) const OBJECT_PROFILE_WORKER_CATALOG_NAME: &str = "object-profile-worker";
pub(crate) const OBJECT_PROFILE_WORKER_BINARY_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_BINARY";
pub(crate) const OBJECT_PROFILE_WORKER_TOKEN_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_TOKEN";
pub(crate) const OBJECT_PROFILE_WORKER_URL_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_URL";
pub(crate) const SCENE_PROFILE_WORKER_CATALOG_NAME: &str = "scene-profile-worker";
pub(crate) const SCENE_PROFILE_WORKER_BINARY_ENV: &str = "A4N_SCENE_PROFILE_WORKER_BINARY";
pub(crate) const SCENE_PROFILE_WORKER_TOKEN_ENV: &str = "A4N_SCENE_PROFILE_WORKER_TOKEN";
pub(crate) const SCENE_PROFILE_WORKER_URL_ENV: &str = "A4N_SCENE_PROFILE_WORKER_URL";
pub(crate) const PROFILE_MATERIAL_WORKER_CATALOG_NAME: &str = "profile-material-worker";
pub(crate) const PROFILE_MATERIAL_WORKER_BINARY_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_BINARY";
pub(crate) const PROFILE_MATERIAL_WORKER_TOKEN_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_TOKEN";
pub(crate) const PROFILE_MATERIAL_WORKER_URL_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_URL";
pub(crate) const PROJECT_STATUS_WORKER_CATALOG_NAME: &str = "project-status-worker";
pub(crate) const PROJECT_STATUS_WORKER_BINARY_ENV: &str = "A4N_PROJECT_STATUS_WORKER_BINARY";
pub(crate) const PROJECT_STATUS_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_STATUS_WORKER_TOKEN";
pub(crate) const PROJECT_STATUS_WORKER_URL_ENV: &str = "A4N_PROJECT_STATUS_WORKER_URL";
pub(crate) const PORTRAIT_WORKER_CATALOG_NAME: &str = "portrait-worker";
pub(crate) const PORTRAIT_WORKER_BINARY_ENV: &str = "A4N_PORTRAIT_WORKER_BINARY";
pub(crate) const PORTRAIT_WORKER_TOKEN_ENV: &str = "A4N_PORTRAIT_WORKER_TOKEN";
pub(crate) const PORTRAIT_WORKER_URL_ENV: &str = "A4N_PORTRAIT_WORKER_URL";
pub(crate) const SOURCE_SNAPSHOT_WORKER_CATALOG_NAME: &str = "source-snapshot-worker";
pub(crate) const SOURCE_SNAPSHOT_WORKER_BINARY_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_BINARY";
pub(crate) const SOURCE_SNAPSHOT_WORKER_TOKEN_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_TOKEN";
pub(crate) const SOURCE_SNAPSHOT_WORKER_URL_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_URL";
pub(crate) const THEME_READ_WORKER_CATALOG_NAME: &str = "theme-read-worker";
pub(crate) const THEME_READ_WORKER_BINARY_ENV: &str = "A4N_THEME_READ_WORKER_BINARY";
pub(crate) const THEME_READ_WORKER_TOKEN_ENV: &str = "A4N_THEME_READ_WORKER_TOKEN";
pub(crate) const THEME_READ_WORKER_URL_ENV: &str = "A4N_THEME_READ_WORKER_URL";
pub(crate) const PROJECT_READ_WORKER_CATALOG_NAME: &str = "project-read-worker";
pub(crate) const PROJECT_READ_WORKER_BINARY_ENV: &str = "A4N_PROJECT_READ_WORKER_BINARY";
pub(crate) const PROJECT_READ_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_READ_WORKER_TOKEN";
pub(crate) const PROJECT_READ_WORKER_URL_ENV: &str = "A4N_PROJECT_READ_WORKER_URL";
pub(crate) const IMPORT_REVIEW_READ_WORKER_CATALOG_NAME: &str = "import-review-read-worker";
pub(crate) const IMPORT_REVIEW_READ_WORKER_BINARY_ENV: &str =
    "A4N_IMPORT_REVIEW_READ_WORKER_BINARY";
pub(crate) const IMPORT_REVIEW_READ_WORKER_TOKEN_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_TOKEN";
pub(crate) const IMPORT_REVIEW_READ_WORKER_URL_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_URL";
pub(crate) const INSPIRATION_HISTORY_WORKER_CATALOG_NAME: &str = "inspiration-history-worker";
pub(crate) const INSPIRATION_HISTORY_WORKER_BINARY_ENV: &str =
    "A4N_INSPIRATION_HISTORY_WORKER_BINARY";
pub(crate) const INSPIRATION_HISTORY_WORKER_TOKEN_ENV: &str =
    "A4N_INSPIRATION_HISTORY_WORKER_TOKEN";
pub(crate) const INSPIRATION_HISTORY_WORKER_URL_ENV: &str = "A4N_INSPIRATION_HISTORY_WORKER_URL";
pub(crate) const PROJECT_ACCESS_WORKER_CATALOG_NAME: &str = "project-access-worker";
pub(crate) const PROJECT_ACCESS_WORKER_BINARY_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_BINARY";
pub(crate) const PROJECT_ACCESS_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_TOKEN";
pub(crate) const PROJECT_ACCESS_WORKER_URL_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_URL";
pub(crate) const PROJECT_COLLABORATION_WORKER_CATALOG_NAME: &str = "project-collaboration-worker";
pub(crate) const PROJECT_COLLABORATION_WORKER_BINARY_ENV: &str =
    "A4N_PROJECT_COLLABORATION_WORKER_BINARY";
pub(crate) const PROJECT_COLLABORATION_WORKER_TOKEN_ENV: &str =
    "A4N_PROJECT_COLLABORATION_WORKER_TOKEN";
pub(crate) const PROJECT_COLLABORATION_WORKER_URL_ENV: &str =
    "A4N_PROJECT_COLLABORATION_WORKER_URL";
pub(crate) const PROJECT_MUTATION_WORKER_CATALOG_NAME: &str = "project-mutation-worker";
pub(crate) const PROJECT_MUTATION_WORKER_BINARY_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_BINARY";
pub(crate) const PROJECT_MUTATION_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_TOKEN";
pub(crate) const PROJECT_MUTATION_WORKER_URL_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_URL";
pub(crate) const CHAPTER_SCENE_WORKER_CATALOG_NAME: &str = "chapter-scene-worker";
pub(crate) const CHAPTER_SCENE_WORKER_BINARY_ENV: &str = "A4N_CHAPTER_SCENE_WORKER_BINARY";
pub(crate) const CHAPTER_SCENE_WORKER_TOKEN_ENV: &str = "A4N_CHAPTER_SCENE_WORKER_TOKEN";
pub(crate) const CHAPTER_SCENE_WORKER_URL_ENV: &str = "A4N_CHAPTER_SCENE_WORKER_URL";
pub(crate) const CHAPTER_CONTENT_WORKER_CATALOG_NAME: &str = "chapter-content-worker";
pub(crate) const CHAPTER_CONTENT_WORKER_BINARY_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_BINARY";
pub(crate) const CHAPTER_CONTENT_WORKER_TOKEN_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_TOKEN";
pub(crate) const CHAPTER_CONTENT_WORKER_URL_ENV: &str = "A4N_CHAPTER_CONTENT_WORKER_URL";
pub(crate) const WORKFLOW_READ_WORKER_CATALOG_NAME: &str = "workflow-read-worker";
pub(crate) const WORKFLOW_READ_WORKER_BINARY_ENV: &str = "A4N_WORKFLOW_READ_WORKER_BINARY";
pub(crate) const WORKFLOW_READ_WORKER_TOKEN_ENV: &str = "A4N_WORKFLOW_READ_WORKER_TOKEN";
pub(crate) const WORKFLOW_READ_WORKER_URL_ENV: &str = "A4N_WORKFLOW_READ_WORKER_URL";
pub(crate) const CONFIG_READ_WORKER_CATALOG_NAME: &str = "config-read-worker";
pub(crate) const CONFIG_READ_WORKER_BINARY_ENV: &str = "A4N_CONFIG_READ_WORKER_BINARY";
pub(crate) const CONFIG_READ_WORKER_TOKEN_ENV: &str = "A4N_CONFIG_READ_WORKER_TOKEN";
pub(crate) const CONFIG_READ_WORKER_URL_ENV: &str = "A4N_CONFIG_READ_WORKER_URL";
pub(crate) const CHARACTER_MUTATION_WORKER_CATALOG_NAME: &str = "character-mutation-worker";
pub(crate) const CHARACTER_MUTATION_WORKER_BINARY_ENV: &str =
    "A4N_CHARACTER_MUTATION_WORKER_BINARY";
pub(crate) const CHARACTER_MUTATION_WORKER_TOKEN_ENV: &str = "A4N_CHARACTER_MUTATION_WORKER_TOKEN";
pub(crate) const CHARACTER_MUTATION_WORKER_URL_ENV: &str = "A4N_CHARACTER_MUTATION_WORKER_URL";
pub(crate) const VIDEO_RECORD_WORKER_CATALOG_NAME: &str = "video-record-worker";
pub(crate) const VIDEO_RECORD_WORKER_BINARY_ENV: &str = "A4N_VIDEO_RECORD_WORKER_BINARY";
pub(crate) const VIDEO_RECORD_WORKER_TOKEN_ENV: &str = "A4N_VIDEO_RECORD_WORKER_TOKEN";
pub(crate) const VIDEO_RECORD_WORKER_URL_ENV: &str = "A4N_VIDEO_RECORD_WORKER_URL";
pub(crate) const MANGA_RECORD_WORKER_CATALOG_NAME: &str = "manga-record-worker";
pub(crate) const MANGA_RECORD_WORKER_BINARY_ENV: &str = "A4N_MANGA_RECORD_WORKER_BINARY";
pub(crate) const MANGA_RECORD_WORKER_TOKEN_ENV: &str = "A4N_MANGA_RECORD_WORKER_TOKEN";
pub(crate) const MANGA_RECORD_WORKER_URL_ENV: &str = "A4N_MANGA_RECORD_WORKER_URL";
pub(crate) const RUNTIME_WORKER_RELOAD_MARKER_ENV: &str = "A4N_RUNTIME_WORKER_RELOAD_MARKER";
pub(crate) const AGENT_WORKER_TOKEN_ENV: &str = "A4N_AGENT_WORKER_TOKEN";
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

pub(crate) fn worker_token() -> anyhow::Result<String> {
    worker_token_for_env(AGENT_WORKER_TOKEN_ENV)
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
