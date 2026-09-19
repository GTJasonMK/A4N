use std::{
    env,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime},
};

use a4n_process_catalog::stateful_worker_catalog_entry_ref;
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    AGENT_WORKER_AUTH_HEADER, WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WorkerCapabilities,
    WorkerHealth, WorkerHealthStatus,
};

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

#[cfg(feature = "test-embedded-processes")]
const CONTROL_PROCESS_ENV: &str = "A4N_CONTROL_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const EDGE_PROCESS_ENV: &str = "A4N_EDGE_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const ARTIFACT_MEDIA_WORKER_PROCESS_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const OUTLINE_WORKER_PROCESS_ENV: &str = "A4N_OUTLINE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const BLUEPRINT_WORKER_PROCESS_ENV: &str = "A4N_BLUEPRINT_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const ASSET_GRAPH_WORKER_PROCESS_ENV: &str = "A4N_ASSET_GRAPH_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const FORESHADOWING_WORKER_PROCESS_ENV: &str = "A4N_FORESHADOWING_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const OBJECT_PROFILE_WORKER_PROCESS_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const SCENE_PROFILE_WORKER_PROCESS_ENV: &str = "A4N_SCENE_PROFILE_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const PROFILE_MATERIAL_WORKER_PROCESS_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const PROJECT_STATUS_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_STATUS_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const PORTRAIT_WORKER_PROCESS_ENV: &str = "A4N_PORTRAIT_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const SOURCE_SNAPSHOT_WORKER_PROCESS_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const THEME_READ_WORKER_PROCESS_ENV: &str = "A4N_THEME_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const PROJECT_READ_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const IMPORT_REVIEW_READ_WORKER_PROCESS_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const INSPIRATION_HISTORY_WORKER_PROCESS_ENV: &str = "A4N_INSPIRATION_HISTORY_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const PROJECT_ACCESS_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const PROJECT_COLLABORATION_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_COLLABORATION_WORKER_PROCESS";
#[cfg(feature = "test-embedded-processes")]
const PROJECT_MUTATION_WORKER_PROCESS_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_PROCESS";
const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";
const EDGE_BINARY_ENV: &str = "A4N_EDGE_BINARY";
const AGENT_WORKER_BINARY_ENV: &str = "A4N_AGENT_WORKER_BINARY";
const AGENT_WORKER_CATALOG_NAME: &str = "agent-worker";
const NOVEL_PLANNING_WORKER_CATALOG_NAME: &str = "novel-planning-worker";
const NOVEL_PLANNING_WORKER_BINARY_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_BINARY";
const NOVEL_PLANNING_WORKER_TOKEN_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_TOKEN";
const NOVEL_PLANNING_WORKER_URL_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_URL";
const ARTIFACT_MEDIA_WORKER_CATALOG_NAME: &str = "artifact-media-worker";
const ARTIFACT_MEDIA_WORKER_BINARY_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_BINARY";
const ARTIFACT_MEDIA_WORKER_TOKEN_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_TOKEN";
const ARTIFACT_MEDIA_WORKER_URL_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_URL";
const OUTLINE_WORKER_CATALOG_NAME: &str = "outline-worker";
const OUTLINE_WORKER_BINARY_ENV: &str = "A4N_OUTLINE_WORKER_BINARY";
const OUTLINE_WORKER_TOKEN_ENV: &str = "A4N_OUTLINE_WORKER_TOKEN";
const OUTLINE_WORKER_URL_ENV: &str = "A4N_OUTLINE_WORKER_URL";
const BLUEPRINT_WORKER_CATALOG_NAME: &str = "blueprint-worker";
const BLUEPRINT_WORKER_BINARY_ENV: &str = "A4N_BLUEPRINT_WORKER_BINARY";
const BLUEPRINT_WORKER_TOKEN_ENV: &str = "A4N_BLUEPRINT_WORKER_TOKEN";
const BLUEPRINT_WORKER_URL_ENV: &str = "A4N_BLUEPRINT_WORKER_URL";
const ASSET_GRAPH_WORKER_CATALOG_NAME: &str = "asset-graph-worker";
const ASSET_GRAPH_WORKER_BINARY_ENV: &str = "A4N_ASSET_GRAPH_WORKER_BINARY";
const ASSET_GRAPH_WORKER_TOKEN_ENV: &str = "A4N_ASSET_GRAPH_WORKER_TOKEN";
const ASSET_GRAPH_WORKER_URL_ENV: &str = "A4N_ASSET_GRAPH_WORKER_URL";
const FORESHADOWING_WORKER_CATALOG_NAME: &str = "foreshadowing-worker";
const FORESHADOWING_WORKER_BINARY_ENV: &str = "A4N_FORESHADOWING_WORKER_BINARY";
const FORESHADOWING_WORKER_TOKEN_ENV: &str = "A4N_FORESHADOWING_WORKER_TOKEN";
const FORESHADOWING_WORKER_URL_ENV: &str = "A4N_FORESHADOWING_WORKER_URL";
const OBJECT_PROFILE_WORKER_CATALOG_NAME: &str = "object-profile-worker";
const OBJECT_PROFILE_WORKER_BINARY_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_BINARY";
const OBJECT_PROFILE_WORKER_TOKEN_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_TOKEN";
const OBJECT_PROFILE_WORKER_URL_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_URL";
const SCENE_PROFILE_WORKER_CATALOG_NAME: &str = "scene-profile-worker";
const SCENE_PROFILE_WORKER_BINARY_ENV: &str = "A4N_SCENE_PROFILE_WORKER_BINARY";
const SCENE_PROFILE_WORKER_TOKEN_ENV: &str = "A4N_SCENE_PROFILE_WORKER_TOKEN";
const SCENE_PROFILE_WORKER_URL_ENV: &str = "A4N_SCENE_PROFILE_WORKER_URL";
const PROFILE_MATERIAL_WORKER_CATALOG_NAME: &str = "profile-material-worker";
const PROFILE_MATERIAL_WORKER_BINARY_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_BINARY";
const PROFILE_MATERIAL_WORKER_TOKEN_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_TOKEN";
const PROFILE_MATERIAL_WORKER_URL_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_URL";
const PROJECT_STATUS_WORKER_CATALOG_NAME: &str = "project-status-worker";
const PROJECT_STATUS_WORKER_BINARY_ENV: &str = "A4N_PROJECT_STATUS_WORKER_BINARY";
const PROJECT_STATUS_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_STATUS_WORKER_TOKEN";
const PROJECT_STATUS_WORKER_URL_ENV: &str = "A4N_PROJECT_STATUS_WORKER_URL";
const PORTRAIT_WORKER_CATALOG_NAME: &str = "portrait-worker";
const PORTRAIT_WORKER_BINARY_ENV: &str = "A4N_PORTRAIT_WORKER_BINARY";
const PORTRAIT_WORKER_TOKEN_ENV: &str = "A4N_PORTRAIT_WORKER_TOKEN";
const PORTRAIT_WORKER_URL_ENV: &str = "A4N_PORTRAIT_WORKER_URL";
const SOURCE_SNAPSHOT_WORKER_CATALOG_NAME: &str = "source-snapshot-worker";
const SOURCE_SNAPSHOT_WORKER_BINARY_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_BINARY";
const SOURCE_SNAPSHOT_WORKER_TOKEN_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_TOKEN";
const SOURCE_SNAPSHOT_WORKER_URL_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_URL";
const THEME_READ_WORKER_CATALOG_NAME: &str = "theme-read-worker";
const THEME_READ_WORKER_BINARY_ENV: &str = "A4N_THEME_READ_WORKER_BINARY";
const THEME_READ_WORKER_TOKEN_ENV: &str = "A4N_THEME_READ_WORKER_TOKEN";
const THEME_READ_WORKER_URL_ENV: &str = "A4N_THEME_READ_WORKER_URL";
const PROJECT_READ_WORKER_CATALOG_NAME: &str = "project-read-worker";
const PROJECT_READ_WORKER_BINARY_ENV: &str = "A4N_PROJECT_READ_WORKER_BINARY";
const PROJECT_READ_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_READ_WORKER_TOKEN";
const PROJECT_READ_WORKER_URL_ENV: &str = "A4N_PROJECT_READ_WORKER_URL";
const IMPORT_REVIEW_READ_WORKER_CATALOG_NAME: &str = "import-review-read-worker";
const IMPORT_REVIEW_READ_WORKER_BINARY_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_BINARY";
const IMPORT_REVIEW_READ_WORKER_TOKEN_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_TOKEN";
const IMPORT_REVIEW_READ_WORKER_URL_ENV: &str = "A4N_IMPORT_REVIEW_READ_WORKER_URL";
const INSPIRATION_HISTORY_WORKER_CATALOG_NAME: &str = "inspiration-history-worker";
const INSPIRATION_HISTORY_WORKER_BINARY_ENV: &str = "A4N_INSPIRATION_HISTORY_WORKER_BINARY";
const INSPIRATION_HISTORY_WORKER_TOKEN_ENV: &str = "A4N_INSPIRATION_HISTORY_WORKER_TOKEN";
const INSPIRATION_HISTORY_WORKER_URL_ENV: &str = "A4N_INSPIRATION_HISTORY_WORKER_URL";
const PROJECT_ACCESS_WORKER_CATALOG_NAME: &str = "project-access-worker";
const PROJECT_ACCESS_WORKER_BINARY_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_BINARY";
const PROJECT_ACCESS_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_TOKEN";
const PROJECT_ACCESS_WORKER_URL_ENV: &str = "A4N_PROJECT_ACCESS_WORKER_URL";
const PROJECT_COLLABORATION_WORKER_CATALOG_NAME: &str = "project-collaboration-worker";
const PROJECT_COLLABORATION_WORKER_BINARY_ENV: &str = "A4N_PROJECT_COLLABORATION_WORKER_BINARY";
const PROJECT_COLLABORATION_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_COLLABORATION_WORKER_TOKEN";
const PROJECT_COLLABORATION_WORKER_URL_ENV: &str = "A4N_PROJECT_COLLABORATION_WORKER_URL";
const PROJECT_MUTATION_WORKER_CATALOG_NAME: &str = "project-mutation-worker";
const PROJECT_MUTATION_WORKER_BINARY_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_BINARY";
const PROJECT_MUTATION_WORKER_TOKEN_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_TOKEN";
const PROJECT_MUTATION_WORKER_URL_ENV: &str = "A4N_PROJECT_MUTATION_WORKER_URL";
const RUNTIME_WORKER_RELOAD_MARKER_ENV: &str = "A4N_RUNTIME_WORKER_RELOAD_MARKER";
const AGENT_WORKER_TOKEN_ENV: &str = "A4N_AGENT_WORKER_TOKEN";
const CONTROL_READY_TIMEOUT: Duration = Duration::from_secs(30);
const CONTROL_READY_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const CONTROL_READY_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
const WORKER_READY_TIMEOUT: Duration = Duration::from_secs(30);
const WORKER_READY_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const WORKER_READY_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
const CHILD_MONITOR_INTERVAL: Duration = Duration::from_millis(100);
const WORKER_RESTART_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BinaryFingerprint {
    modified: SystemTime,
    length: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildWatchEvent {
    EdgeExited(ExitStatus),
    ControlExited(ExitStatus),
    WorkerExited(ExitStatus),
    NovelPlanningWorkerExited(ExitStatus),
    ArtifactMediaWorkerExited(ExitStatus),
    OutlineWorkerExited(ExitStatus),
    BlueprintWorkerExited(ExitStatus),
    AssetGraphWorkerExited(ExitStatus),
    ForeshadowingWorkerExited(ExitStatus),
    ObjectProfileWorkerExited(ExitStatus),
    SceneProfileWorkerExited(ExitStatus),
    ProfileMaterialWorkerExited(ExitStatus),
    ProjectStatusWorkerExited(ExitStatus),
    PortraitWorkerExited(ExitStatus),
    SourceSnapshotWorkerExited(ExitStatus),
    ThemeReadWorkerExited(ExitStatus),
    ProjectReadWorkerExited(ExitStatus),
    ImportReviewReadWorkerExited(ExitStatus),
    InspirationHistoryWorkerExited(ExitStatus),
    ProjectAccessWorkerExited(ExitStatus),
    ProjectCollaborationWorkerExited(ExitStatus),
    ProjectMutationWorkerExited(ExitStatus),
    EdgeChanged,
    ControlChanged,
    WorkerChanged,
    NovelPlanningWorkerChanged,
    ArtifactMediaWorkerChanged,
    OutlineWorkerChanged,
    BlueprintWorkerChanged,
    AssetGraphWorkerChanged,
    ForeshadowingWorkerChanged,
    ObjectProfileWorkerChanged,
    SceneProfileWorkerChanged,
    ProfileMaterialWorkerChanged,
    ProjectStatusWorkerChanged,
    PortraitWorkerChanged,
    SourceSnapshotWorkerChanged,
    ThemeReadWorkerChanged,
    ProjectReadWorkerChanged,
    ImportReviewReadWorkerChanged,
    InspirationHistoryWorkerChanged,
    ProjectAccessWorkerChanged,
    ProjectCollaborationWorkerChanged,
    ProjectMutationWorkerChanged,
    ReloadMarkerChanged,
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "test-embedded-processes")]
    {
        if env::var(CONTROL_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_control::run();
        }
        if env::var(EDGE_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_edge::run();
        }
        if env::var("A4N_AGENT_WORKER_PROCESS").as_deref() == Ok("1") {
            return a4n_agent_worker::run();
        }
        if env::var(ARTIFACT_MEDIA_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_artifact_media_worker::run();
        }
        if env::var(OUTLINE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_outline_worker::run();
        }
        if env::var(BLUEPRINT_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_blueprint_worker::run();
        }
        if env::var(ASSET_GRAPH_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_asset_graph_worker::run();
        }
        if env::var(FORESHADOWING_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_foreshadowing_worker::run();
        }
        if env::var(OBJECT_PROFILE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_object_profile_worker::run();
        }
        if env::var(SCENE_PROFILE_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_scene_profile_worker::run();
        }
        if env::var(PROFILE_MATERIAL_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_profile_material_worker::run();
        }
        if env::var(PROJECT_STATUS_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_project_status_worker::run();
        }
        if env::var(PORTRAIT_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_portrait_worker::run();
        }
        if env::var(SOURCE_SNAPSHOT_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_source_snapshot_worker::run();
        }
        if env::var(THEME_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_theme_read_worker::run();
        }
        if env::var(PROJECT_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_project_read_worker::run();
        }
        if env::var(IMPORT_REVIEW_READ_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_import_review_worker::run();
        }
        if env::var(INSPIRATION_HISTORY_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_inspiration_worker::run();
        }
        if env::var(PROJECT_ACCESS_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_project_access_worker::run();
        }
        if env::var(PROJECT_COLLABORATION_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_project_collaboration_worker::run();
        }
        if env::var(PROJECT_MUTATION_WORKER_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_project_mutation_worker::run();
        }
    }

    let control_addr = control_addr()?;
    let agent_worker_addr = agent_worker_addr()?;
    let worker_token = worker_token()?;
    let novel_planning_worker_addr = novel_planning_worker_addr()?;
    let novel_planning_worker_token = worker_token_for_env(NOVEL_PLANNING_WORKER_TOKEN_ENV)?;
    let artifact_media_worker_addr = artifact_media_worker_addr()?;
    let artifact_media_worker_token = worker_token_for_env(ARTIFACT_MEDIA_WORKER_TOKEN_ENV)?;
    let outline_worker_addr = outline_worker_addr()?;
    let outline_worker_token = worker_token_for_env(OUTLINE_WORKER_TOKEN_ENV)?;
    let blueprint_worker_addr = blueprint_worker_addr()?;
    let blueprint_worker_token = worker_token_for_env(BLUEPRINT_WORKER_TOKEN_ENV)?;
    let asset_graph_worker_addr = asset_graph_worker_addr()?;
    let asset_graph_worker_token = worker_token_for_env(ASSET_GRAPH_WORKER_TOKEN_ENV)?;
    let foreshadowing_worker_addr = foreshadowing_worker_addr()?;
    let foreshadowing_worker_token = worker_token_for_env(FORESHADOWING_WORKER_TOKEN_ENV)?;
    let object_profile_worker_addr = object_profile_worker_addr()?;
    let object_profile_worker_token = worker_token_for_env(OBJECT_PROFILE_WORKER_TOKEN_ENV)?;
    let scene_profile_worker_addr = scene_profile_worker_addr()?;
    let scene_profile_worker_token = worker_token_for_env(SCENE_PROFILE_WORKER_TOKEN_ENV)?;
    let profile_material_worker_addr = profile_material_worker_addr()?;
    let profile_material_worker_token = worker_token_for_env(PROFILE_MATERIAL_WORKER_TOKEN_ENV)?;
    let project_status_worker_addr = project_status_worker_addr()?;
    let project_status_worker_token = worker_token_for_env(PROJECT_STATUS_WORKER_TOKEN_ENV)?;
    let portrait_worker_addr = portrait_worker_addr()?;
    let portrait_worker_token = worker_token_for_env(PORTRAIT_WORKER_TOKEN_ENV)?;
    let source_snapshot_worker_addr = source_snapshot_worker_addr()?;
    let source_snapshot_worker_token = worker_token_for_env(SOURCE_SNAPSHOT_WORKER_TOKEN_ENV)?;
    let theme_read_worker_addr = theme_read_worker_addr()?;
    let theme_read_worker_token = worker_token_for_env(THEME_READ_WORKER_TOKEN_ENV)?;
    let project_read_worker_addr = project_read_worker_addr()?;
    let project_read_worker_token = worker_token_for_env(PROJECT_READ_WORKER_TOKEN_ENV)?;
    let import_review_read_worker_addr = import_review_read_worker_addr()?;
    let import_review_read_worker_token =
        worker_token_for_env(IMPORT_REVIEW_READ_WORKER_TOKEN_ENV)?;
    let inspiration_history_worker_addr = inspiration_history_worker_addr()?;
    let inspiration_history_worker_token =
        worker_token_for_env(INSPIRATION_HISTORY_WORKER_TOKEN_ENV)?;
    let project_access_worker_addr = project_access_worker_addr()?;
    let project_access_worker_token = worker_token_for_env(PROJECT_ACCESS_WORKER_TOKEN_ENV)?;
    let project_collaboration_worker_addr = project_collaboration_worker_addr()?;
    let project_collaboration_worker_token =
        worker_token_for_env(PROJECT_COLLABORATION_WORKER_TOKEN_ENV)?;
    let project_mutation_worker_addr = project_mutation_worker_addr()?;
    let project_mutation_worker_token = worker_token_for_env(PROJECT_MUTATION_WORKER_TOKEN_ENV)?;
    let control_binary = control_binary()?;
    let mut control_fingerprint = binary_fingerprint(&control_binary)?;
    let worker_binary = agent_worker_binary()?;
    let mut worker_fingerprint = binary_fingerprint(&worker_binary)?;
    let edge_binary = edge_binary()?;
    let mut edge_fingerprint = binary_fingerprint(&edge_binary)?;
    let novel_planning_worker_binary = novel_planning_worker_binary()?;
    let mut novel_planning_worker_fingerprint = binary_fingerprint(&novel_planning_worker_binary)?;
    let artifact_media_worker_binary = artifact_media_worker_binary()?;
    let mut artifact_media_worker_fingerprint = binary_fingerprint(&artifact_media_worker_binary)?;
    let outline_worker_binary = outline_worker_binary()?;
    let mut outline_worker_fingerprint = binary_fingerprint(&outline_worker_binary)?;
    let blueprint_worker_binary = blueprint_worker_binary()?;
    let mut blueprint_worker_fingerprint = binary_fingerprint(&blueprint_worker_binary)?;
    let asset_graph_worker_binary = asset_graph_worker_binary()?;
    let mut asset_graph_worker_fingerprint = binary_fingerprint(&asset_graph_worker_binary)?;
    let foreshadowing_worker_binary = foreshadowing_worker_binary()?;
    let mut foreshadowing_worker_fingerprint = binary_fingerprint(&foreshadowing_worker_binary)?;
    let object_profile_worker_binary = object_profile_worker_binary()?;
    let mut object_profile_worker_fingerprint = binary_fingerprint(&object_profile_worker_binary)?;
    let scene_profile_worker_binary = scene_profile_worker_binary()?;
    let mut scene_profile_worker_fingerprint = binary_fingerprint(&scene_profile_worker_binary)?;
    let profile_material_worker_binary = profile_material_worker_binary()?;
    let mut profile_material_worker_fingerprint =
        binary_fingerprint(&profile_material_worker_binary)?;
    let project_status_worker_binary = project_status_worker_binary()?;
    let mut project_status_worker_fingerprint = binary_fingerprint(&project_status_worker_binary)?;
    let portrait_worker_binary = portrait_worker_binary()?;
    let mut portrait_worker_fingerprint = binary_fingerprint(&portrait_worker_binary)?;
    let source_snapshot_worker_binary = source_snapshot_worker_binary()?;
    let mut source_snapshot_worker_fingerprint =
        binary_fingerprint(&source_snapshot_worker_binary)?;
    let theme_read_worker_binary = theme_read_worker_binary()?;
    let import_review_read_worker_binary = import_review_read_worker_binary()?;
    let inspiration_history_worker_binary = inspiration_history_worker_binary()?;
    let project_access_worker_binary = project_access_worker_binary()?;
    let project_collaboration_worker_binary = project_collaboration_worker_binary()?;
    let project_mutation_worker_binary = project_mutation_worker_binary()?;
    let reload_marker = reload_marker_path();
    let mut reload_marker_fingerprint = optional_binary_fingerprint(reload_marker.as_deref())?;
    let mut artifact_media_worker = spawn_artifact_media_worker(
        &artifact_media_worker_binary,
        artifact_media_worker_addr,
        &artifact_media_worker_token,
    )?;
    if let Err(error) = wait_for_artifact_media_worker(
        &mut artifact_media_worker,
        artifact_media_worker_addr,
        &artifact_media_worker_token,
    ) {
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut novel_planning_worker = match spawn_novel_planning_worker(
        &novel_planning_worker_binary,
        novel_planning_worker_addr,
        &novel_planning_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_novel_planning_worker(
        &mut novel_planning_worker,
        novel_planning_worker_addr,
        &novel_planning_worker_token,
    ) {
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut outline_worker = match spawn_outline_worker(
        &outline_worker_binary,
        outline_worker_addr,
        &outline_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_outline_worker(
        &mut outline_worker,
        outline_worker_addr,
        &outline_worker_token,
    ) {
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut blueprint_worker = match spawn_blueprint_worker(
        &blueprint_worker_binary,
        blueprint_worker_addr,
        &blueprint_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_blueprint_worker(
        &mut blueprint_worker,
        blueprint_worker_addr,
        &blueprint_worker_token,
    ) {
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut asset_graph_worker = match spawn_asset_graph_worker(
        &asset_graph_worker_binary,
        asset_graph_worker_addr,
        &asset_graph_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_asset_graph_worker(
        &mut asset_graph_worker,
        asset_graph_worker_addr,
        &asset_graph_worker_token,
    ) {
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut foreshadowing_worker = match spawn_foreshadowing_worker(
        &foreshadowing_worker_binary,
        foreshadowing_worker_addr,
        &foreshadowing_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_foreshadowing_worker(
        &mut foreshadowing_worker,
        foreshadowing_worker_addr,
        &foreshadowing_worker_token,
    ) {
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut object_profile_worker = match spawn_object_profile_worker(
        &object_profile_worker_binary,
        object_profile_worker_addr,
        &object_profile_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_object_profile_worker(
        &mut object_profile_worker,
        object_profile_worker_addr,
        &object_profile_worker_token,
    ) {
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut scene_profile_worker = match spawn_scene_profile_worker(
        &scene_profile_worker_binary,
        scene_profile_worker_addr,
        &scene_profile_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_scene_profile_worker(
        &mut scene_profile_worker,
        scene_profile_worker_addr,
        &scene_profile_worker_token,
    ) {
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut portrait_worker = match spawn_portrait_worker(
        &portrait_worker_binary,
        portrait_worker_addr,
        &portrait_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_portrait_worker(
        &mut portrait_worker,
        portrait_worker_addr,
        &portrait_worker_token,
    ) {
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut source_snapshot_worker = match spawn_source_snapshot_worker(
        &source_snapshot_worker_binary,
        source_snapshot_worker_addr,
        &source_snapshot_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_source_snapshot_worker(
        &mut source_snapshot_worker,
        source_snapshot_worker_addr,
        &source_snapshot_worker_token,
    ) {
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        return Err(error);
    }
    let mut control = match spawn_control(
        &control_binary,
        control_addr,
        &worker_token,
        novel_planning_worker_addr,
        &novel_planning_worker_token,
        artifact_media_worker_addr,
        &artifact_media_worker_token,
        outline_worker_addr,
        &outline_worker_token,
        blueprint_worker_addr,
        &blueprint_worker_token,
        asset_graph_worker_addr,
        &asset_graph_worker_token,
        foreshadowing_worker_addr,
        &foreshadowing_worker_token,
        object_profile_worker_addr,
        &object_profile_worker_token,
        scene_profile_worker_addr,
        &scene_profile_worker_token,
        profile_material_worker_addr,
        &profile_material_worker_token,
        project_status_worker_addr,
        &project_status_worker_token,
        portrait_worker_addr,
        &portrait_worker_token,
        source_snapshot_worker_addr,
        &source_snapshot_worker_token,
        theme_read_worker_addr,
        &theme_read_worker_token,
        project_read_worker_addr,
        &project_read_worker_token,
        import_review_read_worker_addr,
        &import_review_read_worker_token,
        inspiration_history_worker_addr,
        &inspiration_history_worker_token,
        project_access_worker_addr,
        &project_access_worker_token,
        project_collaboration_worker_addr,
        &project_collaboration_worker_token,
        project_mutation_worker_addr,
        &project_mutation_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_control(&mut control, control_addr) {
        let cleanup_result = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return match cleanup_result {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(anyhow::anyhow!(
                "A4N Control 未就绪: {error:#}；清理 Control 失败: {cleanup_error:#}"
            )),
        };
    }
    let mut agent_worker = match spawn_agent_worker(
        &worker_binary,
        control_addr,
        agent_worker_addr,
        &worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_agent_worker(&mut agent_worker, agent_worker_addr, &worker_token) {
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut profile_material_worker = match spawn_profile_material_worker(
        &profile_material_worker_binary,
        profile_material_worker_addr,
        &profile_material_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_profile_material_worker(
        &mut profile_material_worker,
        profile_material_worker_addr,
        &profile_material_worker_token,
    ) {
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut project_status_worker = match spawn_project_status_worker(
        &project_status_worker_binary,
        project_status_worker_addr,
        &project_status_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_status_worker(
        &mut project_status_worker,
        project_status_worker_addr,
        &project_status_worker_token,
    ) {
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut theme_read_worker = match spawn_theme_read_worker(
        &theme_read_worker_binary,
        theme_read_worker_addr,
        &theme_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_theme_read_worker(
        &mut theme_read_worker,
        theme_read_worker_addr,
        &theme_read_worker_token,
    ) {
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut theme_read_worker_fingerprint = binary_fingerprint(&theme_read_worker_binary)?;
    let project_read_worker_binary = project_read_worker_binary()?;
    let mut project_read_worker = match spawn_project_read_worker(
        &project_read_worker_binary,
        project_read_worker_addr,
        &project_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_read_worker(
        &mut project_read_worker,
        project_read_worker_addr,
        &project_read_worker_token,
    ) {
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut project_read_worker_fingerprint = binary_fingerprint(&project_read_worker_binary)?;
    let mut import_review_read_worker = match spawn_import_review_read_worker(
        &import_review_read_worker_binary,
        import_review_read_worker_addr,
        &import_review_read_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_import_review_read_worker(
        &mut import_review_read_worker,
        import_review_read_worker_addr,
        &import_review_read_worker_token,
    ) {
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut import_review_read_worker_fingerprint =
        binary_fingerprint(&import_review_read_worker_binary)?;
    let mut inspiration_history_worker = match spawn_inspiration_history_worker(
        &inspiration_history_worker_binary,
        inspiration_history_worker_addr,
        &inspiration_history_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_inspiration_history_worker(
        &mut inspiration_history_worker,
        inspiration_history_worker_addr,
        &inspiration_history_worker_token,
    ) {
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut inspiration_history_worker_fingerprint =
        binary_fingerprint(&inspiration_history_worker_binary)?;
    let mut project_access_worker = match spawn_project_access_worker(
        &project_access_worker_binary,
        project_access_worker_addr,
        &project_access_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_access_worker(
        &mut project_access_worker,
        project_access_worker_addr,
        &project_access_worker_token,
    ) {
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut project_access_worker_fingerprint = binary_fingerprint(&project_access_worker_binary)?;
    let mut project_collaboration_worker = match spawn_project_collaboration_worker(
        &project_collaboration_worker_binary,
        project_collaboration_worker_addr,
        &project_collaboration_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_collaboration_worker(
        &mut project_collaboration_worker,
        project_collaboration_worker_addr,
        &project_collaboration_worker_token,
    ) {
        let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut project_collaboration_worker_fingerprint =
        binary_fingerprint(&project_collaboration_worker_binary)?;
    let mut project_mutation_worker = match spawn_project_mutation_worker(
        &project_mutation_worker_binary,
        project_mutation_worker_addr,
        &project_mutation_worker_token,
    ) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    if let Err(error) = wait_for_project_mutation_worker(
        &mut project_mutation_worker,
        project_mutation_worker_addr,
        &project_mutation_worker_token,
    ) {
        let _ = stop_project_mutation_worker(&mut project_mutation_worker);
        let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
        let _ = stop_project_access_worker(&mut project_access_worker);
        let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
        let _ = stop_import_review_read_worker(&mut import_review_read_worker);
        let _ = stop_project_read_worker(&mut project_read_worker);
        let _ = stop_theme_read_worker(&mut theme_read_worker);
        let _ = stop_project_status_worker(&mut project_status_worker);
        let _ = stop_profile_material_worker(&mut profile_material_worker);
        let _ = stop_agent_worker(&mut agent_worker);
        let _ = stop_control(&mut control);
        let _ = stop_artifact_media_worker(&mut artifact_media_worker);
        let _ = stop_novel_planning_worker(&mut novel_planning_worker);
        let _ = stop_outline_worker(&mut outline_worker);
        let _ = stop_blueprint_worker(&mut blueprint_worker);
        let _ = stop_asset_graph_worker(&mut asset_graph_worker);
        let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
        let _ = stop_object_profile_worker(&mut object_profile_worker);
        let _ = stop_scene_profile_worker(&mut scene_profile_worker);
        let _ = stop_portrait_worker(&mut portrait_worker);
        let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
        return Err(error);
    }
    let mut project_mutation_worker_fingerprint =
        binary_fingerprint(&project_mutation_worker_binary)?;
    let mut edge = match spawn_edge(&edge_binary, control_addr) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_agent_worker(&mut agent_worker);
            let _ = stop_control(&mut control);
            let _ = stop_artifact_media_worker(&mut artifact_media_worker);
            let _ = stop_novel_planning_worker(&mut novel_planning_worker);
            let _ = stop_outline_worker(&mut outline_worker);
            let _ = stop_blueprint_worker(&mut blueprint_worker);
            let _ = stop_asset_graph_worker(&mut asset_graph_worker);
            let _ = stop_foreshadowing_worker(&mut foreshadowing_worker);
            let _ = stop_object_profile_worker(&mut object_profile_worker);
            let _ = stop_scene_profile_worker(&mut scene_profile_worker);
            let _ = stop_profile_material_worker(&mut profile_material_worker);
            let _ = stop_project_status_worker(&mut project_status_worker);
            let _ = stop_theme_read_worker(&mut theme_read_worker);
            let _ = stop_project_read_worker(&mut project_read_worker);
            let _ = stop_import_review_read_worker(&mut import_review_read_worker);
            let _ = stop_inspiration_history_worker(&mut inspiration_history_worker);
            let _ = stop_project_access_worker(&mut project_access_worker);
            let _ = stop_project_collaboration_worker(&mut project_collaboration_worker);
            let _ = stop_project_mutation_worker(&mut project_mutation_worker);
            let _ = stop_portrait_worker(&mut portrait_worker);
            let _ = stop_source_snapshot_worker(&mut source_snapshot_worker);
            return Err(error);
        }
    };
    let watch_result = supervise_runtime_children(
        &mut edge,
        &mut control,
        &mut agent_worker,
        control_addr,
        agent_worker_addr,
        &worker_token,
        &control_binary,
        &mut control_fingerprint,
        &worker_binary,
        &mut worker_fingerprint,
        novel_planning_worker_addr,
        &novel_planning_worker_token,
        &mut novel_planning_worker,
        &novel_planning_worker_binary,
        &mut novel_planning_worker_fingerprint,
        artifact_media_worker_addr,
        &artifact_media_worker_token,
        &mut artifact_media_worker,
        &artifact_media_worker_binary,
        &mut artifact_media_worker_fingerprint,
        outline_worker_addr,
        &outline_worker_token,
        &mut outline_worker,
        &outline_worker_binary,
        &mut outline_worker_fingerprint,
        blueprint_worker_addr,
        &blueprint_worker_token,
        &mut blueprint_worker,
        &blueprint_worker_binary,
        &mut blueprint_worker_fingerprint,
        asset_graph_worker_addr,
        &asset_graph_worker_token,
        &mut asset_graph_worker,
        &asset_graph_worker_binary,
        &mut asset_graph_worker_fingerprint,
        foreshadowing_worker_addr,
        &foreshadowing_worker_token,
        &mut foreshadowing_worker,
        &foreshadowing_worker_binary,
        &mut foreshadowing_worker_fingerprint,
        object_profile_worker_addr,
        &object_profile_worker_token,
        &mut object_profile_worker,
        &object_profile_worker_binary,
        &mut object_profile_worker_fingerprint,
        scene_profile_worker_addr,
        &scene_profile_worker_token,
        &mut scene_profile_worker,
        &scene_profile_worker_binary,
        &mut scene_profile_worker_fingerprint,
        profile_material_worker_addr,
        &profile_material_worker_token,
        &mut profile_material_worker,
        &profile_material_worker_binary,
        &mut profile_material_worker_fingerprint,
        project_status_worker_addr,
        &project_status_worker_token,
        &mut project_status_worker,
        &project_status_worker_binary,
        &mut project_status_worker_fingerprint,
        portrait_worker_addr,
        &portrait_worker_token,
        &mut portrait_worker,
        &portrait_worker_binary,
        &mut portrait_worker_fingerprint,
        source_snapshot_worker_addr,
        &source_snapshot_worker_token,
        &mut source_snapshot_worker,
        &source_snapshot_worker_binary,
        &mut source_snapshot_worker_fingerprint,
        theme_read_worker_addr,
        &theme_read_worker_token,
        &mut theme_read_worker,
        &theme_read_worker_binary,
        &mut theme_read_worker_fingerprint,
        project_read_worker_addr,
        &project_read_worker_token,
        &mut project_read_worker,
        &project_read_worker_binary,
        &mut project_read_worker_fingerprint,
        import_review_read_worker_addr,
        &import_review_read_worker_token,
        &mut import_review_read_worker,
        &import_review_read_worker_binary,
        &mut import_review_read_worker_fingerprint,
        inspiration_history_worker_addr,
        &inspiration_history_worker_token,
        &mut inspiration_history_worker,
        &inspiration_history_worker_binary,
        &mut inspiration_history_worker_fingerprint,
        project_access_worker_addr,
        &project_access_worker_token,
        &mut project_access_worker,
        &project_access_worker_binary,
        &mut project_access_worker_fingerprint,
        project_collaboration_worker_addr,
        &project_collaboration_worker_token,
        &mut project_collaboration_worker,
        &project_collaboration_worker_binary,
        &mut project_collaboration_worker_fingerprint,
        project_mutation_worker_addr,
        &project_mutation_worker_token,
        &mut project_mutation_worker,
        &project_mutation_worker_binary,
        &mut project_mutation_worker_fingerprint,
        &edge_binary,
        &mut edge_fingerprint,
        reload_marker.as_deref(),
        &mut reload_marker_fingerprint,
    );
    let edge_result = stop_edge(&mut edge).and(watch_result.map(|_| ()));
    let worker_result = stop_agent_worker(&mut agent_worker);
    let control_result = stop_control(&mut control);
    let novel_planning_worker_result = stop_novel_planning_worker(&mut novel_planning_worker);
    let artifact_media_worker_result = stop_artifact_media_worker(&mut artifact_media_worker);
    let outline_worker_result = stop_outline_worker(&mut outline_worker);
    let blueprint_worker_result = stop_blueprint_worker(&mut blueprint_worker);
    let asset_graph_worker_result = stop_asset_graph_worker(&mut asset_graph_worker);
    let foreshadowing_worker_result = stop_foreshadowing_worker(&mut foreshadowing_worker);
    let object_profile_worker_result = stop_object_profile_worker(&mut object_profile_worker);
    let scene_profile_worker_result = stop_scene_profile_worker(&mut scene_profile_worker);
    let profile_material_worker_result = stop_profile_material_worker(&mut profile_material_worker);
    let project_status_worker_result = stop_project_status_worker(&mut project_status_worker);
    let portrait_worker_result = stop_portrait_worker(&mut portrait_worker);
    let source_snapshot_worker_result = stop_source_snapshot_worker(&mut source_snapshot_worker);
    let theme_read_worker_result = stop_theme_read_worker(&mut theme_read_worker);
    let project_read_worker_result = stop_project_read_worker(&mut project_read_worker);
    let import_review_read_worker_result =
        stop_import_review_read_worker(&mut import_review_read_worker);
    let inspiration_history_worker_result =
        stop_inspiration_history_worker(&mut inspiration_history_worker);
    let project_access_worker_result = stop_project_access_worker(&mut project_access_worker);
    let project_collaboration_worker_result =
        stop_project_collaboration_worker(&mut project_collaboration_worker);
    let project_mutation_worker_result = stop_project_mutation_worker(&mut project_mutation_worker);
    finish_shutdown(
        edge_result,
        worker_result,
        control_result,
        novel_planning_worker_result,
        artifact_media_worker_result,
        outline_worker_result,
        blueprint_worker_result,
        asset_graph_worker_result,
        foreshadowing_worker_result,
        object_profile_worker_result,
        scene_profile_worker_result,
        profile_material_worker_result,
        project_status_worker_result,
        portrait_worker_result,
        source_snapshot_worker_result,
        theme_read_worker_result,
        project_read_worker_result,
        import_review_read_worker_result,
        inspiration_history_worker_result,
        project_access_worker_result,
        project_collaboration_worker_result,
        project_mutation_worker_result,
    )
}

fn supervise_runtime_children(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_addr: SocketAddr,
    agent_worker_addr: SocketAddr,
    worker_token: &str,
    control_binary: &Path,
    control_fingerprint: &mut BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: &mut BinaryFingerprint,
    novel_planning_worker_addr: SocketAddr,
    novel_planning_worker_token: &str,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: &mut BinaryFingerprint,
    artifact_media_worker_addr: SocketAddr,
    artifact_media_worker_token: &str,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: &mut BinaryFingerprint,
    outline_worker_addr: SocketAddr,
    outline_worker_token: &str,
    outline_worker: &mut Child,
    outline_worker_binary: &Path,
    outline_worker_fingerprint: &mut BinaryFingerprint,
    blueprint_worker_addr: SocketAddr,
    blueprint_worker_token: &str,
    blueprint_worker: &mut Child,
    blueprint_worker_binary: &Path,
    blueprint_worker_fingerprint: &mut BinaryFingerprint,
    asset_graph_worker_addr: SocketAddr,
    asset_graph_worker_token: &str,
    asset_graph_worker: &mut Child,
    asset_graph_worker_binary: &Path,
    asset_graph_worker_fingerprint: &mut BinaryFingerprint,
    foreshadowing_worker_addr: SocketAddr,
    foreshadowing_worker_token: &str,
    foreshadowing_worker: &mut Child,
    foreshadowing_worker_binary: &Path,
    foreshadowing_worker_fingerprint: &mut BinaryFingerprint,
    object_profile_worker_addr: SocketAddr,
    object_profile_worker_token: &str,
    object_profile_worker: &mut Child,
    object_profile_worker_binary: &Path,
    object_profile_worker_fingerprint: &mut BinaryFingerprint,
    scene_profile_worker_addr: SocketAddr,
    scene_profile_worker_token: &str,
    scene_profile_worker: &mut Child,
    scene_profile_worker_binary: &Path,
    scene_profile_worker_fingerprint: &mut BinaryFingerprint,
    profile_material_worker_addr: SocketAddr,
    profile_material_worker_token: &str,
    profile_material_worker: &mut Child,
    profile_material_worker_binary: &Path,
    profile_material_worker_fingerprint: &mut BinaryFingerprint,
    project_status_worker_addr: SocketAddr,
    project_status_worker_token: &str,
    project_status_worker: &mut Child,
    project_status_worker_binary: &Path,
    project_status_worker_fingerprint: &mut BinaryFingerprint,
    portrait_worker_addr: SocketAddr,
    portrait_worker_token: &str,
    portrait_worker: &mut Child,
    portrait_worker_binary: &Path,
    portrait_worker_fingerprint: &mut BinaryFingerprint,
    source_snapshot_worker_addr: SocketAddr,
    source_snapshot_worker_token: &str,
    source_snapshot_worker: &mut Child,
    source_snapshot_worker_binary: &Path,
    source_snapshot_worker_fingerprint: &mut BinaryFingerprint,
    theme_read_worker_addr: SocketAddr,
    theme_read_worker_token: &str,
    theme_read_worker: &mut Child,
    theme_read_worker_binary: &Path,
    theme_read_worker_fingerprint: &mut BinaryFingerprint,
    project_read_worker_addr: SocketAddr,
    project_read_worker_token: &str,
    project_read_worker: &mut Child,
    project_read_worker_binary: &Path,
    project_read_worker_fingerprint: &mut BinaryFingerprint,
    import_review_read_worker_addr: SocketAddr,
    import_review_read_worker_token: &str,
    import_review_read_worker: &mut Child,
    import_review_read_worker_binary: &Path,
    import_review_read_worker_fingerprint: &mut BinaryFingerprint,
    inspiration_history_worker_addr: SocketAddr,
    inspiration_history_worker_token: &str,
    inspiration_history_worker: &mut Child,
    inspiration_history_worker_binary: &Path,
    inspiration_history_worker_fingerprint: &mut BinaryFingerprint,
    project_access_worker_addr: SocketAddr,
    project_access_worker_token: &str,
    project_access_worker: &mut Child,
    project_access_worker_binary: &Path,
    project_access_worker_fingerprint: &mut BinaryFingerprint,
    project_collaboration_worker_addr: SocketAddr,
    project_collaboration_worker_token: &str,
    project_collaboration_worker: &mut Child,
    project_collaboration_worker_binary: &Path,
    project_collaboration_worker_fingerprint: &mut BinaryFingerprint,
    project_mutation_worker_addr: SocketAddr,
    project_mutation_worker_token: &str,
    project_mutation_worker: &mut Child,
    project_mutation_worker_binary: &Path,
    project_mutation_worker_fingerprint: &mut BinaryFingerprint,
    edge_binary: &Path,
    edge_fingerprint: &mut BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: &mut Option<BinaryFingerprint>,
) -> anyhow::Result<Option<String>> {
    loop {
        let Some(event) = wait_for_edge_or_child_with_mutation(
            edge,
            control,
            agent_worker,
            control_binary,
            *control_fingerprint,
            worker_binary,
            *worker_fingerprint,
            novel_planning_worker,
            novel_planning_worker_binary,
            *novel_planning_worker_fingerprint,
            artifact_media_worker,
            artifact_media_worker_binary,
            *artifact_media_worker_fingerprint,
            Some((
                outline_worker,
                outline_worker_binary,
                *outline_worker_fingerprint,
            )),
            Some((
                blueprint_worker,
                blueprint_worker_binary,
                *blueprint_worker_fingerprint,
            )),
            Some((
                asset_graph_worker,
                asset_graph_worker_binary,
                *asset_graph_worker_fingerprint,
            )),
            Some((
                foreshadowing_worker,
                foreshadowing_worker_binary,
                *foreshadowing_worker_fingerprint,
            )),
            Some((
                object_profile_worker,
                object_profile_worker_binary,
                *object_profile_worker_fingerprint,
            )),
            Some((
                scene_profile_worker,
                scene_profile_worker_binary,
                *scene_profile_worker_fingerprint,
            )),
            Some((
                profile_material_worker,
                profile_material_worker_binary,
                *profile_material_worker_fingerprint,
            )),
            Some((
                project_status_worker,
                project_status_worker_binary,
                *project_status_worker_fingerprint,
            )),
            Some((
                portrait_worker,
                portrait_worker_binary,
                *portrait_worker_fingerprint,
            )),
            Some((
                source_snapshot_worker,
                source_snapshot_worker_binary,
                *source_snapshot_worker_fingerprint,
            )),
            Some((
                theme_read_worker,
                theme_read_worker_binary,
                *theme_read_worker_fingerprint,
            )),
            Some((
                project_read_worker,
                project_read_worker_binary,
                *project_read_worker_fingerprint,
            )),
            Some((
                import_review_read_worker,
                import_review_read_worker_binary,
                *import_review_read_worker_fingerprint,
            )),
            Some((
                inspiration_history_worker,
                inspiration_history_worker_binary,
                *inspiration_history_worker_fingerprint,
            )),
            Some((
                project_access_worker,
                project_access_worker_binary,
                *project_access_worker_fingerprint,
            )),
            Some((
                project_collaboration_worker,
                project_collaboration_worker_binary,
                *project_collaboration_worker_fingerprint,
            )),
            Some((
                project_mutation_worker,
                project_mutation_worker_binary,
                *project_mutation_worker_fingerprint,
            )),
            edge_binary,
            *edge_fingerprint,
            reload_marker,
            *reload_marker_fingerprint,
        )?
        else {
            return Ok(None);
        };

        match event {
            ChildWatchEvent::EdgeExited(status) => {
                eprintln!("A4N Edge 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *edge = spawn_edge(edge_binary, control_addr)?;
                *edge_fingerprint = binary_fingerprint(edge_binary)?;
            }
            ChildWatchEvent::ControlExited(status) => {
                eprintln!("A4N Control 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *control = spawn_control(
                    control_binary,
                    control_addr,
                    worker_token,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                    outline_worker_addr,
                    outline_worker_token,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                    project_status_worker_addr,
                    project_status_worker_token,
                    portrait_worker_addr,
                    portrait_worker_token,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                    project_read_worker_addr,
                    project_read_worker_token,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                    project_access_worker_addr,
                    project_access_worker_token,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                wait_for_control(control, control_addr)?;
                *control_fingerprint = binary_fingerprint(control_binary)?;
            }
            ChildWatchEvent::WorkerExited(status) => {
                eprintln!("A4N Agent worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *agent_worker = spawn_agent_worker(
                    worker_binary,
                    control_addr,
                    agent_worker_addr,
                    worker_token,
                )?;
                wait_for_agent_worker(agent_worker, agent_worker_addr, worker_token)?;
                *worker_fingerprint = binary_fingerprint(worker_binary)?;
            }
            ChildWatchEvent::NovelPlanningWorkerExited(status) => {
                eprintln!("A4N Novel Planning worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *novel_planning_worker = spawn_novel_planning_worker(
                    novel_planning_worker_binary,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                )?;
                wait_for_novel_planning_worker(
                    novel_planning_worker,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                )?;
                *novel_planning_worker_fingerprint =
                    binary_fingerprint(novel_planning_worker_binary)?;
            }
            ChildWatchEvent::ArtifactMediaWorkerExited(status) => {
                eprintln!("A4N Artifact/Media worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *artifact_media_worker = spawn_artifact_media_worker(
                    artifact_media_worker_binary,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                )?;
                wait_for_artifact_media_worker(
                    artifact_media_worker,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                )?;
                *artifact_media_worker_fingerprint =
                    binary_fingerprint(artifact_media_worker_binary)?;
            }
            ChildWatchEvent::OutlineWorkerExited(status) => {
                eprintln!("A4N Outline worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *outline_worker = spawn_outline_worker(
                    outline_worker_binary,
                    outline_worker_addr,
                    outline_worker_token,
                )?;
                wait_for_outline_worker(outline_worker, outline_worker_addr, outline_worker_token)?;
                *outline_worker_fingerprint = binary_fingerprint(outline_worker_binary)?;
            }
            ChildWatchEvent::BlueprintWorkerExited(status) => {
                eprintln!("A4N Blueprint worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *blueprint_worker = spawn_blueprint_worker(
                    blueprint_worker_binary,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                )?;
                wait_for_blueprint_worker(
                    blueprint_worker,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                )?;
                *blueprint_worker_fingerprint = binary_fingerprint(blueprint_worker_binary)?;
            }
            ChildWatchEvent::AssetGraphWorkerExited(status) => {
                eprintln!("A4N Asset Graph worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *asset_graph_worker = spawn_asset_graph_worker(
                    asset_graph_worker_binary,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                )?;
                wait_for_asset_graph_worker(
                    asset_graph_worker,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                )?;
                *asset_graph_worker_fingerprint = binary_fingerprint(asset_graph_worker_binary)?;
            }
            ChildWatchEvent::ForeshadowingWorkerExited(status) => {
                eprintln!("A4N Foreshadowing worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *foreshadowing_worker = spawn_foreshadowing_worker(
                    foreshadowing_worker_binary,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                )?;
                wait_for_foreshadowing_worker(
                    foreshadowing_worker,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                )?;
                *foreshadowing_worker_fingerprint =
                    binary_fingerprint(foreshadowing_worker_binary)?;
            }
            ChildWatchEvent::ObjectProfileWorkerExited(status) => {
                eprintln!("A4N Object Profile worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *object_profile_worker = spawn_object_profile_worker(
                    object_profile_worker_binary,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                )?;
                wait_for_object_profile_worker(
                    object_profile_worker,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                )?;
                *object_profile_worker_fingerprint =
                    binary_fingerprint(object_profile_worker_binary)?;
            }
            ChildWatchEvent::SceneProfileWorkerExited(status) => {
                eprintln!("A4N Scene Profile worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *scene_profile_worker = spawn_scene_profile_worker(
                    scene_profile_worker_binary,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                )?;
                wait_for_scene_profile_worker(
                    scene_profile_worker,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                )?;
                *scene_profile_worker_fingerprint =
                    binary_fingerprint(scene_profile_worker_binary)?;
            }
            ChildWatchEvent::ProfileMaterialWorkerExited(status) => {
                eprintln!("A4N Profile Material worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *profile_material_worker = spawn_profile_material_worker(
                    profile_material_worker_binary,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                )?;
                wait_for_profile_material_worker(
                    profile_material_worker,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                )?;
                *profile_material_worker_fingerprint =
                    binary_fingerprint(profile_material_worker_binary)?;
            }
            ChildWatchEvent::ProjectStatusWorkerExited(status) => {
                eprintln!("A4N Project Status worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_status_worker = spawn_project_status_worker(
                    project_status_worker_binary,
                    project_status_worker_addr,
                    project_status_worker_token,
                )?;
                wait_for_project_status_worker(
                    project_status_worker,
                    project_status_worker_addr,
                    project_status_worker_token,
                )?;
                *project_status_worker_fingerprint =
                    binary_fingerprint(project_status_worker_binary)?;
            }
            ChildWatchEvent::PortraitWorkerExited(status) => {
                eprintln!("A4N Portrait worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *portrait_worker = spawn_portrait_worker(
                    portrait_worker_binary,
                    portrait_worker_addr,
                    portrait_worker_token,
                )?;
                wait_for_portrait_worker(
                    portrait_worker,
                    portrait_worker_addr,
                    portrait_worker_token,
                )?;
                *portrait_worker_fingerprint = binary_fingerprint(portrait_worker_binary)?;
            }
            ChildWatchEvent::SourceSnapshotWorkerExited(status) => {
                eprintln!("A4N Source Snapshot worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *source_snapshot_worker = spawn_source_snapshot_worker(
                    source_snapshot_worker_binary,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                )?;
                wait_for_source_snapshot_worker(
                    source_snapshot_worker,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                )?;
                *source_snapshot_worker_fingerprint =
                    binary_fingerprint(source_snapshot_worker_binary)?;
            }
            ChildWatchEvent::ThemeReadWorkerExited(status) => {
                eprintln!("A4N Theme Read worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *theme_read_worker = spawn_theme_read_worker(
                    theme_read_worker_binary,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                )?;
                wait_for_theme_read_worker(
                    theme_read_worker,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                )?;
                *theme_read_worker_fingerprint = binary_fingerprint(theme_read_worker_binary)?;
            }
            ChildWatchEvent::ProjectReadWorkerExited(status) => {
                eprintln!("A4N Project Read worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_read_worker = spawn_project_read_worker(
                    project_read_worker_binary,
                    project_read_worker_addr,
                    project_read_worker_token,
                )?;
                wait_for_project_read_worker(
                    project_read_worker,
                    project_read_worker_addr,
                    project_read_worker_token,
                )?;
                *project_read_worker_fingerprint = binary_fingerprint(project_read_worker_binary)?;
            }
            ChildWatchEvent::ImportReviewReadWorkerExited(status) => {
                eprintln!("A4N Import Review Read worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *import_review_read_worker = spawn_import_review_read_worker(
                    import_review_read_worker_binary,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                )?;
                wait_for_import_review_read_worker(
                    import_review_read_worker,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                )?;
                *import_review_read_worker_fingerprint =
                    binary_fingerprint(import_review_read_worker_binary)?;
            }
            ChildWatchEvent::InspirationHistoryWorkerExited(status) => {
                eprintln!("A4N Inspiration History worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *inspiration_history_worker = spawn_inspiration_history_worker(
                    inspiration_history_worker_binary,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                )?;
                wait_for_inspiration_history_worker(
                    inspiration_history_worker,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                )?;
                *inspiration_history_worker_fingerprint =
                    binary_fingerprint(inspiration_history_worker_binary)?;
            }
            ChildWatchEvent::ProjectAccessWorkerExited(status) => {
                eprintln!("A4N Project Access worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_access_worker = spawn_project_access_worker(
                    project_access_worker_binary,
                    project_access_worker_addr,
                    project_access_worker_token,
                )?;
                wait_for_project_access_worker(
                    project_access_worker,
                    project_access_worker_addr,
                    project_access_worker_token,
                )?;
                *project_access_worker_fingerprint =
                    binary_fingerprint(project_access_worker_binary)?;
            }
            ChildWatchEvent::ProjectCollaborationWorkerExited(status) => {
                eprintln!("A4N Project Collaboration worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_collaboration_worker = spawn_project_collaboration_worker(
                    project_collaboration_worker_binary,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                )?;
                wait_for_project_collaboration_worker(
                    project_collaboration_worker,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                )?;
                *project_collaboration_worker_fingerprint =
                    binary_fingerprint(project_collaboration_worker_binary)?;
            }
            ChildWatchEvent::ProjectMutationWorkerExited(status) => {
                eprintln!("A4N Project Mutation worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_mutation_worker = spawn_project_mutation_worker(
                    project_mutation_worker_binary,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                wait_for_project_mutation_worker(
                    project_mutation_worker,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                *project_mutation_worker_fingerprint =
                    binary_fingerprint(project_mutation_worker_binary)?;
            }
            ChildWatchEvent::EdgeChanged => {
                eprintln!("检测到 A4N Edge 构建完成；只替换 Edge");
                stop_edge(edge)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *edge = spawn_edge(edge_binary, control_addr)?;
                *edge_fingerprint = binary_fingerprint(edge_binary)?;
            }
            ChildWatchEvent::ControlChanged => {
                eprintln!("检测到 A4N Control 构建完成；只替换 Control");
                stop_control(control)?;
                *control = spawn_control(
                    control_binary,
                    control_addr,
                    worker_token,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                    outline_worker_addr,
                    outline_worker_token,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                    project_status_worker_addr,
                    project_status_worker_token,
                    portrait_worker_addr,
                    portrait_worker_token,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                    project_read_worker_addr,
                    project_read_worker_token,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                    project_access_worker_addr,
                    project_access_worker_token,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                wait_for_control(control, control_addr)?;
                *control_fingerprint = binary_fingerprint(control_binary)?;
            }
            ChildWatchEvent::WorkerChanged => {
                eprintln!("检测到 A4N Agent worker 构建完成；只替换 Agent worker");
                stop_agent_worker(agent_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *agent_worker = spawn_agent_worker(
                    worker_binary,
                    control_addr,
                    agent_worker_addr,
                    worker_token,
                )?;
                wait_for_agent_worker(agent_worker, agent_worker_addr, worker_token)?;
                *worker_fingerprint = binary_fingerprint(worker_binary)?;
            }
            ChildWatchEvent::NovelPlanningWorkerChanged => {
                eprintln!(
                    "检测到 A4N Novel Planning worker 构建完成；只替换 Novel Planning worker"
                );
                stop_novel_planning_worker(novel_planning_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *novel_planning_worker = spawn_novel_planning_worker(
                    novel_planning_worker_binary,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                )?;
                wait_for_novel_planning_worker(
                    novel_planning_worker,
                    novel_planning_worker_addr,
                    novel_planning_worker_token,
                )?;
                *novel_planning_worker_fingerprint =
                    binary_fingerprint(novel_planning_worker_binary)?;
            }
            ChildWatchEvent::ArtifactMediaWorkerChanged => {
                eprintln!(
                    "检测到 A4N Artifact/Media worker 构建完成；只替换 Artifact/Media worker"
                );
                stop_artifact_media_worker(artifact_media_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *artifact_media_worker = spawn_artifact_media_worker(
                    artifact_media_worker_binary,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                )?;
                wait_for_artifact_media_worker(
                    artifact_media_worker,
                    artifact_media_worker_addr,
                    artifact_media_worker_token,
                )?;
                *artifact_media_worker_fingerprint =
                    binary_fingerprint(artifact_media_worker_binary)?;
            }
            ChildWatchEvent::OutlineWorkerChanged => {
                eprintln!("检测到 A4N Outline worker 构建完成；只替换 Outline worker");
                stop_outline_worker(outline_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *outline_worker = spawn_outline_worker(
                    outline_worker_binary,
                    outline_worker_addr,
                    outline_worker_token,
                )?;
                wait_for_outline_worker(outline_worker, outline_worker_addr, outline_worker_token)?;
                *outline_worker_fingerprint = binary_fingerprint(outline_worker_binary)?;
            }
            ChildWatchEvent::BlueprintWorkerChanged => {
                eprintln!("检测到 A4N Blueprint worker 构建完成；只替换 Blueprint worker");
                stop_blueprint_worker(blueprint_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *blueprint_worker = spawn_blueprint_worker(
                    blueprint_worker_binary,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                )?;
                wait_for_blueprint_worker(
                    blueprint_worker,
                    blueprint_worker_addr,
                    blueprint_worker_token,
                )?;
                *blueprint_worker_fingerprint = binary_fingerprint(blueprint_worker_binary)?;
            }
            ChildWatchEvent::AssetGraphWorkerChanged => {
                eprintln!("检测到 A4N Asset Graph worker 构建完成；只替换 Asset Graph worker");
                stop_asset_graph_worker(asset_graph_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *asset_graph_worker = spawn_asset_graph_worker(
                    asset_graph_worker_binary,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                )?;
                wait_for_asset_graph_worker(
                    asset_graph_worker,
                    asset_graph_worker_addr,
                    asset_graph_worker_token,
                )?;
                *asset_graph_worker_fingerprint = binary_fingerprint(asset_graph_worker_binary)?;
            }
            ChildWatchEvent::ForeshadowingWorkerChanged => {
                eprintln!("检测到 A4N Foreshadowing worker 构建完成；只替换 Foreshadowing worker");
                stop_foreshadowing_worker(foreshadowing_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *foreshadowing_worker = spawn_foreshadowing_worker(
                    foreshadowing_worker_binary,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                )?;
                wait_for_foreshadowing_worker(
                    foreshadowing_worker,
                    foreshadowing_worker_addr,
                    foreshadowing_worker_token,
                )?;
                *foreshadowing_worker_fingerprint =
                    binary_fingerprint(foreshadowing_worker_binary)?;
            }
            ChildWatchEvent::ObjectProfileWorkerChanged => {
                eprintln!(
                    "检测到 A4N Object Profile worker 构建完成；只替换 Object Profile worker"
                );
                stop_object_profile_worker(object_profile_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *object_profile_worker = spawn_object_profile_worker(
                    object_profile_worker_binary,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                )?;
                wait_for_object_profile_worker(
                    object_profile_worker,
                    object_profile_worker_addr,
                    object_profile_worker_token,
                )?;
                *object_profile_worker_fingerprint =
                    binary_fingerprint(object_profile_worker_binary)?;
            }
            ChildWatchEvent::SceneProfileWorkerChanged => {
                eprintln!("检测到 A4N Scene Profile worker 构建完成；只替换 Scene Profile worker");
                stop_scene_profile_worker(scene_profile_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *scene_profile_worker = spawn_scene_profile_worker(
                    scene_profile_worker_binary,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                )?;
                wait_for_scene_profile_worker(
                    scene_profile_worker,
                    scene_profile_worker_addr,
                    scene_profile_worker_token,
                )?;
                *scene_profile_worker_fingerprint =
                    binary_fingerprint(scene_profile_worker_binary)?;
            }
            ChildWatchEvent::ProfileMaterialWorkerChanged => {
                eprintln!(
                    "检测到 A4N Profile Material worker 构建完成；只替换 Profile Material worker"
                );
                stop_profile_material_worker(profile_material_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *profile_material_worker = spawn_profile_material_worker(
                    profile_material_worker_binary,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                )?;
                wait_for_profile_material_worker(
                    profile_material_worker,
                    profile_material_worker_addr,
                    profile_material_worker_token,
                )?;
                *profile_material_worker_fingerprint =
                    binary_fingerprint(profile_material_worker_binary)?;
            }
            ChildWatchEvent::ProjectStatusWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Status worker 构建完成；只替换 Project Status worker"
                );
                stop_project_status_worker(project_status_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_status_worker = spawn_project_status_worker(
                    project_status_worker_binary,
                    project_status_worker_addr,
                    project_status_worker_token,
                )?;
                wait_for_project_status_worker(
                    project_status_worker,
                    project_status_worker_addr,
                    project_status_worker_token,
                )?;
                *project_status_worker_fingerprint =
                    binary_fingerprint(project_status_worker_binary)?;
            }
            ChildWatchEvent::PortraitWorkerChanged => {
                eprintln!("检测到 A4N Portrait worker 构建完成；只替换 Portrait worker");
                stop_portrait_worker(portrait_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *portrait_worker = spawn_portrait_worker(
                    portrait_worker_binary,
                    portrait_worker_addr,
                    portrait_worker_token,
                )?;
                wait_for_portrait_worker(
                    portrait_worker,
                    portrait_worker_addr,
                    portrait_worker_token,
                )?;
                *portrait_worker_fingerprint = binary_fingerprint(portrait_worker_binary)?;
            }
            ChildWatchEvent::SourceSnapshotWorkerChanged => {
                eprintln!(
                    "检测到 A4N Source Snapshot worker 构建完成；只替换 Source Snapshot worker"
                );
                stop_source_snapshot_worker(source_snapshot_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *source_snapshot_worker = spawn_source_snapshot_worker(
                    source_snapshot_worker_binary,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                )?;
                wait_for_source_snapshot_worker(
                    source_snapshot_worker,
                    source_snapshot_worker_addr,
                    source_snapshot_worker_token,
                )?;
                *source_snapshot_worker_fingerprint =
                    binary_fingerprint(source_snapshot_worker_binary)?;
            }
            ChildWatchEvent::ThemeReadWorkerChanged => {
                eprintln!("检测到 A4N Theme Read worker 构建完成；只替换 Theme Read worker");
                stop_theme_read_worker(theme_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *theme_read_worker = spawn_theme_read_worker(
                    theme_read_worker_binary,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                )?;
                wait_for_theme_read_worker(
                    theme_read_worker,
                    theme_read_worker_addr,
                    theme_read_worker_token,
                )?;
                *theme_read_worker_fingerprint = binary_fingerprint(theme_read_worker_binary)?;
            }
            ChildWatchEvent::ProjectReadWorkerChanged => {
                eprintln!("检测到 A4N Project Read worker 构建完成；只替换 Project Read worker");
                stop_project_read_worker(project_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_read_worker = spawn_project_read_worker(
                    project_read_worker_binary,
                    project_read_worker_addr,
                    project_read_worker_token,
                )?;
                wait_for_project_read_worker(
                    project_read_worker,
                    project_read_worker_addr,
                    project_read_worker_token,
                )?;
                *project_read_worker_fingerprint = binary_fingerprint(project_read_worker_binary)?;
            }
            ChildWatchEvent::ImportReviewReadWorkerChanged => {
                eprintln!(
                    "检测到 A4N Import Review Read worker 构建完成；只替换 Import Review Read worker"
                );
                stop_import_review_read_worker(import_review_read_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *import_review_read_worker = spawn_import_review_read_worker(
                    import_review_read_worker_binary,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                )?;
                wait_for_import_review_read_worker(
                    import_review_read_worker,
                    import_review_read_worker_addr,
                    import_review_read_worker_token,
                )?;
                *import_review_read_worker_fingerprint =
                    binary_fingerprint(import_review_read_worker_binary)?;
            }
            ChildWatchEvent::InspirationHistoryWorkerChanged => {
                eprintln!(
                    "检测到 A4N Inspiration History worker 构建完成；只替换 Inspiration History worker"
                );
                stop_inspiration_history_worker(inspiration_history_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *inspiration_history_worker = spawn_inspiration_history_worker(
                    inspiration_history_worker_binary,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                )?;
                wait_for_inspiration_history_worker(
                    inspiration_history_worker,
                    inspiration_history_worker_addr,
                    inspiration_history_worker_token,
                )?;
                *inspiration_history_worker_fingerprint =
                    binary_fingerprint(inspiration_history_worker_binary)?;
            }
            ChildWatchEvent::ProjectAccessWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Access worker 构建完成；只替换 Project Access worker"
                );
                stop_project_access_worker(project_access_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_access_worker = spawn_project_access_worker(
                    project_access_worker_binary,
                    project_access_worker_addr,
                    project_access_worker_token,
                )?;
                wait_for_project_access_worker(
                    project_access_worker,
                    project_access_worker_addr,
                    project_access_worker_token,
                )?;
                *project_access_worker_fingerprint =
                    binary_fingerprint(project_access_worker_binary)?;
            }
            ChildWatchEvent::ProjectCollaborationWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Collaboration worker 构建完成；只替换 Project Collaboration worker"
                );
                stop_project_collaboration_worker(project_collaboration_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_collaboration_worker = spawn_project_collaboration_worker(
                    project_collaboration_worker_binary,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                )?;
                wait_for_project_collaboration_worker(
                    project_collaboration_worker,
                    project_collaboration_worker_addr,
                    project_collaboration_worker_token,
                )?;
                *project_collaboration_worker_fingerprint =
                    binary_fingerprint(project_collaboration_worker_binary)?;
            }
            ChildWatchEvent::ProjectMutationWorkerChanged => {
                eprintln!(
                    "检测到 A4N Project Mutation worker 构建完成；只替换 Project Mutation worker"
                );
                stop_project_mutation_worker(project_mutation_worker)?;
                std::thread::sleep(WORKER_RESTART_DELAY);
                *project_mutation_worker = spawn_project_mutation_worker(
                    project_mutation_worker_binary,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                wait_for_project_mutation_worker(
                    project_mutation_worker,
                    project_mutation_worker_addr,
                    project_mutation_worker_token,
                )?;
                *project_mutation_worker_fingerprint =
                    binary_fingerprint(project_mutation_worker_binary)?;
            }
            ChildWatchEvent::ReloadMarkerChanged => {
                // The marker is a publication hint from the development watcher.
                // The binary fingerprints above decide which process actually
                // needs replacement; a marker-only change must not restart either.
                *reload_marker_fingerprint = optional_binary_fingerprint(reload_marker)?;
            }
        }
    }
}

#[cfg(test)]
fn wait_for_edge_or_child(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    wait_for_edge_or_child_with_outline(
        edge,
        control,
        agent_worker,
        control_binary,
        control_fingerprint,
        worker_binary,
        worker_fingerprint,
        novel_planning_worker,
        novel_planning_worker_binary,
        novel_planning_worker_fingerprint,
        artifact_media_worker,
        artifact_media_worker_binary,
        artifact_media_worker_fingerprint,
        None,
        None,
        None,
        edge_binary,
        edge_fingerprint,
        reload_marker,
        reload_marker_fingerprint,
    )
}

#[cfg(test)]
fn wait_for_edge_or_child_with_outline(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    outline_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    blueprint_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    asset_graph_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    wait_for_edge_or_child_with_mutation(
        edge,
        control,
        agent_worker,
        control_binary,
        control_fingerprint,
        worker_binary,
        worker_fingerprint,
        novel_planning_worker,
        novel_planning_worker_binary,
        novel_planning_worker_fingerprint,
        artifact_media_worker,
        artifact_media_worker_binary,
        artifact_media_worker_fingerprint,
        outline_worker,
        blueprint_worker,
        asset_graph_worker,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        edge_binary,
        edge_fingerprint,
        reload_marker,
        reload_marker_fingerprint,
    )
}

#[cfg(test)]
fn wait_for_edge_or_child_with_workers(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    outline_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    blueprint_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    asset_graph_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    foreshadowing_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    object_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    scene_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    profile_material_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_status_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    portrait_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    source_snapshot_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    theme_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    import_review_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    inspiration_history_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_access_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    project_collaboration_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    wait_for_edge_or_child_with_mutation(
        edge,
        control,
        agent_worker,
        control_binary,
        control_fingerprint,
        worker_binary,
        worker_fingerprint,
        novel_planning_worker,
        novel_planning_worker_binary,
        novel_planning_worker_fingerprint,
        artifact_media_worker,
        artifact_media_worker_binary,
        artifact_media_worker_fingerprint,
        outline_worker,
        blueprint_worker,
        asset_graph_worker,
        foreshadowing_worker,
        object_profile_worker,
        scene_profile_worker,
        profile_material_worker,
        project_status_worker,
        portrait_worker,
        source_snapshot_worker,
        theme_read_worker,
        project_read_worker,
        import_review_read_worker,
        inspiration_history_worker,
        project_access_worker,
        project_collaboration_worker,
        None,
        edge_binary,
        edge_fingerprint,
        reload_marker,
        reload_marker_fingerprint,
    )
}

fn wait_for_edge_or_child_with_mutation(
    edge: &mut Child,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
    novel_planning_worker: &mut Child,
    novel_planning_worker_binary: &Path,
    novel_planning_worker_fingerprint: BinaryFingerprint,
    artifact_media_worker: &mut Child,
    artifact_media_worker_binary: &Path,
    artifact_media_worker_fingerprint: BinaryFingerprint,
    mut outline_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut blueprint_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut asset_graph_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut foreshadowing_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut object_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut scene_profile_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut profile_material_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_status_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut portrait_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut source_snapshot_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut theme_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut import_review_read_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut inspiration_history_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_access_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_collaboration_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    mut project_mutation_worker: Option<(&mut Child, &Path, BinaryFingerprint)>,
    edge_binary: &Path,
    edge_fingerprint: BinaryFingerprint,
    reload_marker: Option<&Path>,
    reload_marker_fingerprint: Option<BinaryFingerprint>,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    loop {
        if let Some(status) = edge.try_wait()? {
            return Ok(Some(ChildWatchEvent::EdgeExited(status)));
        }
        if let Some(status) = control.try_wait()? {
            return Ok(Some(ChildWatchEvent::ControlExited(status)));
        }
        if let Some(status) = agent_worker.try_wait()? {
            return Ok(Some(ChildWatchEvent::WorkerExited(status)));
        }
        if let Some(status) = novel_planning_worker.try_wait()? {
            return Ok(Some(ChildWatchEvent::NovelPlanningWorkerExited(status)));
        }
        if let Some(status) = artifact_media_worker.try_wait()? {
            return Ok(Some(ChildWatchEvent::ArtifactMediaWorkerExited(status)));
        }
        if let Some((outline_worker, _, _)) = outline_worker.as_mut()
            && let Some(status) = outline_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::OutlineWorkerExited(status)));
        }
        if let Some((blueprint_worker, _, _)) = blueprint_worker.as_mut()
            && let Some(status) = blueprint_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::BlueprintWorkerExited(status)));
        }
        if let Some((asset_graph_worker, _, _)) = asset_graph_worker.as_mut()
            && let Some(status) = asset_graph_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::AssetGraphWorkerExited(status)));
        }
        if let Some((foreshadowing_worker, _, _)) = foreshadowing_worker.as_mut()
            && let Some(status) = foreshadowing_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ForeshadowingWorkerExited(status)));
        }
        if let Some((object_profile_worker, _, _)) = object_profile_worker.as_mut()
            && let Some(status) = object_profile_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ObjectProfileWorkerExited(status)));
        }
        if let Some((scene_profile_worker, _, _)) = scene_profile_worker.as_mut()
            && let Some(status) = scene_profile_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::SceneProfileWorkerExited(status)));
        }
        if let Some((profile_material_worker, _, _)) = profile_material_worker.as_mut()
            && let Some(status) = profile_material_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProfileMaterialWorkerExited(status)));
        }
        if let Some((project_status_worker, _, _)) = project_status_worker.as_mut()
            && let Some(status) = project_status_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectStatusWorkerExited(status)));
        }
        if let Some((portrait_worker, _, _)) = portrait_worker.as_mut()
            && let Some(status) = portrait_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::PortraitWorkerExited(status)));
        }
        if let Some((source_snapshot_worker, _, _)) = source_snapshot_worker.as_mut()
            && let Some(status) = source_snapshot_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::SourceSnapshotWorkerExited(status)));
        }
        if let Some((theme_read_worker, _, _)) = theme_read_worker.as_mut()
            && let Some(status) = theme_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ThemeReadWorkerExited(status)));
        }
        if let Some((project_read_worker, _, _)) = project_read_worker.as_mut()
            && let Some(status) = project_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectReadWorkerExited(status)));
        }
        if let Some((import_review_read_worker, _, _)) = import_review_read_worker.as_mut()
            && let Some(status) = import_review_read_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ImportReviewReadWorkerExited(status)));
        }
        if let Some((inspiration_history_worker, _, _)) = inspiration_history_worker.as_mut()
            && let Some(status) = inspiration_history_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::InspirationHistoryWorkerExited(
                status,
            )));
        }
        if let Some((project_access_worker, _, _)) = project_access_worker.as_mut()
            && let Some(status) = project_access_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectAccessWorkerExited(status)));
        }
        if let Some((project_collaboration_worker, _, _)) = project_collaboration_worker.as_mut()
            && let Some(status) = project_collaboration_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectCollaborationWorkerExited(
                status,
            )));
        }
        if let Some((project_mutation_worker, _, _)) = project_mutation_worker.as_mut()
            && let Some(status) = project_mutation_worker.try_wait()?
        {
            return Ok(Some(ChildWatchEvent::ProjectMutationWorkerExited(status)));
        }
        if binary_fingerprint(edge_binary)? != edge_fingerprint {
            return Ok(Some(ChildWatchEvent::EdgeChanged));
        }
        let control_changed = binary_fingerprint(control_binary)? != control_fingerprint;
        let worker_changed = binary_fingerprint(worker_binary)? != worker_fingerprint;
        let novel_planning_worker_changed =
            binary_fingerprint(novel_planning_worker_binary)? != novel_planning_worker_fingerprint;
        if control_changed {
            return Ok(Some(ChildWatchEvent::ControlChanged));
        }
        if worker_changed {
            return Ok(Some(ChildWatchEvent::WorkerChanged));
        }
        if novel_planning_worker_changed {
            return Ok(Some(ChildWatchEvent::NovelPlanningWorkerChanged));
        }
        let artifact_media_worker_changed =
            binary_fingerprint(artifact_media_worker_binary)? != artifact_media_worker_fingerprint;
        if artifact_media_worker_changed {
            return Ok(Some(ChildWatchEvent::ArtifactMediaWorkerChanged));
        }
        if let Some((_, outline_worker_binary, outline_worker_fingerprint)) =
            outline_worker.as_ref()
            && binary_fingerprint(outline_worker_binary)? != *outline_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::OutlineWorkerChanged));
        }
        if let Some((_, blueprint_worker_binary, blueprint_worker_fingerprint)) =
            blueprint_worker.as_ref()
            && binary_fingerprint(blueprint_worker_binary)? != *blueprint_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::BlueprintWorkerChanged));
        }
        if let Some((_, asset_graph_worker_binary, asset_graph_worker_fingerprint)) =
            asset_graph_worker.as_ref()
            && binary_fingerprint(asset_graph_worker_binary)? != *asset_graph_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::AssetGraphWorkerChanged));
        }
        if let Some((_, foreshadowing_worker_binary, foreshadowing_worker_fingerprint)) =
            foreshadowing_worker.as_ref()
            && binary_fingerprint(foreshadowing_worker_binary)? != *foreshadowing_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ForeshadowingWorkerChanged));
        }
        if let Some((_, object_profile_worker_binary, object_profile_worker_fingerprint)) =
            object_profile_worker.as_ref()
            && binary_fingerprint(object_profile_worker_binary)?
                != *object_profile_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ObjectProfileWorkerChanged));
        }
        if let Some((_, scene_profile_worker_binary, scene_profile_worker_fingerprint)) =
            scene_profile_worker.as_ref()
            && binary_fingerprint(scene_profile_worker_binary)? != *scene_profile_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::SceneProfileWorkerChanged));
        }
        if let Some((_, profile_material_worker_binary, profile_material_worker_fingerprint)) =
            profile_material_worker.as_ref()
            && binary_fingerprint(profile_material_worker_binary)?
                != *profile_material_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProfileMaterialWorkerChanged));
        }
        if let Some((_, project_status_worker_binary, project_status_worker_fingerprint)) =
            project_status_worker.as_ref()
            && binary_fingerprint(project_status_worker_binary)?
                != *project_status_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectStatusWorkerChanged));
        }
        if let Some((_, portrait_worker_binary, portrait_worker_fingerprint)) =
            portrait_worker.as_ref()
            && binary_fingerprint(portrait_worker_binary)? != *portrait_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::PortraitWorkerChanged));
        }
        if let Some((_, source_snapshot_worker_binary, source_snapshot_worker_fingerprint)) =
            source_snapshot_worker.as_ref()
            && binary_fingerprint(source_snapshot_worker_binary)?
                != *source_snapshot_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::SourceSnapshotWorkerChanged));
        }
        if let Some((_, theme_read_worker_binary, theme_read_worker_fingerprint)) =
            theme_read_worker.as_ref()
            && binary_fingerprint(theme_read_worker_binary)? != *theme_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ThemeReadWorkerChanged));
        }
        if let Some((_, project_read_worker_binary, project_read_worker_fingerprint)) =
            project_read_worker.as_ref()
            && binary_fingerprint(project_read_worker_binary)? != *project_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectReadWorkerChanged));
        }
        if let Some((_, import_review_read_worker_binary, import_review_read_worker_fingerprint)) =
            import_review_read_worker.as_ref()
            && binary_fingerprint(import_review_read_worker_binary)?
                != *import_review_read_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ImportReviewReadWorkerChanged));
        }
        if let Some((_, inspiration_history_worker_binary, inspiration_history_worker_fingerprint)) =
            inspiration_history_worker.as_ref()
            && binary_fingerprint(inspiration_history_worker_binary)?
                != *inspiration_history_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::InspirationHistoryWorkerChanged));
        }
        if let Some((_, project_access_worker_binary, project_access_worker_fingerprint)) =
            project_access_worker.as_ref()
            && binary_fingerprint(project_access_worker_binary)?
                != *project_access_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectAccessWorkerChanged));
        }
        if let Some((
            _,
            project_collaboration_worker_binary,
            project_collaboration_worker_fingerprint,
        )) = project_collaboration_worker.as_ref()
            && binary_fingerprint(project_collaboration_worker_binary)?
                != *project_collaboration_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectCollaborationWorkerChanged));
        }
        if let Some((_, project_mutation_worker_binary, project_mutation_worker_fingerprint)) =
            project_mutation_worker.as_ref()
            && binary_fingerprint(project_mutation_worker_binary)?
                != *project_mutation_worker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ProjectMutationWorkerChanged));
        }
        if reload_marker.is_some()
            && optional_binary_fingerprint(reload_marker)? != reload_marker_fingerprint
        {
            return Ok(Some(ChildWatchEvent::ReloadMarkerChanged));
        }
        std::thread::sleep(CHILD_MONITOR_INTERVAL);
    }
}

fn binary_fingerprint(path: &Path) -> anyhow::Result<BinaryFingerprint> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| anyhow::anyhow!("读取 worker 二进制 {} 失败: {error}", path.display()))?;
    let modified = metadata.modified().map_err(|error| {
        anyhow::anyhow!(
            "读取 worker 二进制 {} 修改时间失败: {error}",
            path.display()
        )
    })?;
    Ok(BinaryFingerprint {
        modified,
        length: metadata.len(),
    })
}

fn optional_binary_fingerprint(path: Option<&Path>) -> anyhow::Result<Option<BinaryFingerprint>> {
    let Some(path) = path else {
        return Ok(None);
    };
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(BinaryFingerprint {
            modified: metadata.modified().map_err(|error| {
                anyhow::anyhow!(
                    "读取 reload marker {} 修改时间失败: {error}",
                    path.display()
                )
            })?,
            length: metadata.len(),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(anyhow::anyhow!(
            "读取 reload marker {} 失败: {error}",
            path.display()
        )),
    }
}

fn reload_marker_path() -> Option<PathBuf> {
    env::var_os(RUNTIME_WORKER_RELOAD_MARKER_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn wait_for_control(child: &mut Child, address: SocketAddr) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut last_error = None;
    while started.elapsed() < CONTROL_READY_TIMEOUT {
        if let Some(status) = child.try_wait()? {
            anyhow::bail!(
                "A4N Control 在 listener 就绪前退出: {status}; 最后一次连接错误: {}",
                last_error
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "无".to_string())
            );
        }
        match TcpStream::connect_timeout(&address, CONTROL_READY_CONNECT_TIMEOUT) {
            Ok(_) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        std::thread::sleep(CONTROL_READY_RETRY_INTERVAL);
    }

    anyhow::bail!(
        "A4N Control 在 {} 秒内未监听 {address}; 最后一次连接错误: {}",
        CONTROL_READY_TIMEOUT.as_secs(),
        last_error
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "无".to_string())
    )
}

fn web_addr() -> anyhow::Result<SocketAddr> {
    env::var("A4N_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("A4N_WEB_ADDR 无效: {error}"))
}

fn control_addr() -> anyhow::Result<SocketAddr> {
    if let Some(value) = env::var_os("A4N_CONTROL_ADDR") {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("A4N_CONTROL_ADDR 必须是有效 UTF-8 地址"))?;
        let address = value
            .parse()
            .map_err(|error| anyhow::anyhow!("A4N_CONTROL_ADDR 无效: {error}"))?;
        ensure_nonzero_port("A4N_CONTROL_ADDR", address)
    } else {
        control_addr_from_web(web_addr()?)
    }
}

fn control_addr_from_web(web_addr: SocketAddr) -> anyhow::Result<SocketAddr> {
    if web_addr.port() == 0 {
        return Ok(SocketAddr::from(([127, 0, 0, 1], 18080)));
    }
    let control_port = web_addr
        .port()
        .checked_add(1)
        .filter(|port| *port != 0)
        .unwrap_or(18080);
    Ok(SocketAddr::from(([127, 0, 0, 1], control_port)))
}

fn agent_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = agent_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn agent_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(AGENT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            AGENT_WORKER_CATALOG_NAME
        )
    })
}

fn novel_planning_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(NOVEL_PLANNING_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            NOVEL_PLANNING_WORKER_CATALOG_NAME
        )
    })
}

fn novel_planning_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = novel_planning_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn artifact_media_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(ARTIFACT_MEDIA_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            ARTIFACT_MEDIA_WORKER_CATALOG_NAME
        )
    })
}

fn artifact_media_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = artifact_media_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn outline_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(OUTLINE_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            OUTLINE_WORKER_CATALOG_NAME
        )
    })
}

fn outline_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = outline_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn blueprint_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(BLUEPRINT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            BLUEPRINT_WORKER_CATALOG_NAME
        )
    })
}

fn blueprint_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = blueprint_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn asset_graph_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(ASSET_GRAPH_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            ASSET_GRAPH_WORKER_CATALOG_NAME
        )
    })
}

fn asset_graph_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = asset_graph_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn foreshadowing_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(FORESHADOWING_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            FORESHADOWING_WORKER_CATALOG_NAME
        )
    })
}

fn foreshadowing_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = foreshadowing_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn object_profile_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(OBJECT_PROFILE_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            OBJECT_PROFILE_WORKER_CATALOG_NAME
        )
    })
}

fn object_profile_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = object_profile_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn scene_profile_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(SCENE_PROFILE_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            SCENE_PROFILE_WORKER_CATALOG_NAME
        )
    })
}

fn scene_profile_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = scene_profile_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn profile_material_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROFILE_MATERIAL_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROFILE_MATERIAL_WORKER_CATALOG_NAME
        )
    })
}

fn profile_material_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = profile_material_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn project_status_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_STATUS_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_STATUS_WORKER_CATALOG_NAME
        )
    })
}

fn project_status_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_status_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn portrait_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PORTRAIT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PORTRAIT_WORKER_CATALOG_NAME
        )
    })
}

fn portrait_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = portrait_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn source_snapshot_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(SOURCE_SNAPSHOT_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            SOURCE_SNAPSHOT_WORKER_CATALOG_NAME
        )
    })
}

fn source_snapshot_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = source_snapshot_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn theme_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(THEME_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            THEME_READ_WORKER_CATALOG_NAME
        )
    })
}

fn theme_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = theme_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn project_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_READ_WORKER_CATALOG_NAME
        )
    })
}

fn project_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn import_review_read_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(IMPORT_REVIEW_READ_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            IMPORT_REVIEW_READ_WORKER_CATALOG_NAME
        )
    })
}

fn import_review_read_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = import_review_read_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn inspiration_history_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(INSPIRATION_HISTORY_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            INSPIRATION_HISTORY_WORKER_CATALOG_NAME
        )
    })
}

fn inspiration_history_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = inspiration_history_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn project_access_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_ACCESS_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_ACCESS_WORKER_CATALOG_NAME
        )
    })
}

fn project_access_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_access_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn project_collaboration_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_COLLABORATION_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_COLLABORATION_WORKER_CATALOG_NAME
        )
    })
}

fn project_collaboration_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_collaboration_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn project_mutation_worker_metadata()
-> anyhow::Result<&'static a4n_process_catalog::StatefulWorkerCatalogEntry> {
    stateful_worker_catalog_entry_ref(PROJECT_MUTATION_WORKER_CATALOG_NAME).ok_or_else(|| {
        anyhow::anyhow!(
            "进程目录缺少 {} 的 stateful worker 元数据",
            PROJECT_MUTATION_WORKER_CATALOG_NAME
        )
    })
}

fn project_mutation_worker_addr() -> anyhow::Result<SocketAddr> {
    let metadata = project_mutation_worker_metadata()?;
    let value = env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
    let address = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
    ensure_nonzero_port(&metadata.addr_env, address)
}

fn ensure_nonzero_port(name: &str, address: SocketAddr) -> anyhow::Result<SocketAddr> {
    if address.port() == 0 {
        anyhow::bail!("{name} 不允许使用端口 0；Edge 必须知道 Control 的固定地址");
    }
    Ok(address)
}

fn spawn_control(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
    novel_planning_worker_addr: SocketAddr,
    novel_planning_worker_token: &str,
    artifact_media_worker_addr: SocketAddr,
    artifact_media_worker_token: &str,
    outline_worker_addr: SocketAddr,
    outline_worker_token: &str,
    blueprint_worker_addr: SocketAddr,
    blueprint_worker_token: &str,
    asset_graph_worker_addr: SocketAddr,
    asset_graph_worker_token: &str,
    foreshadowing_worker_addr: SocketAddr,
    foreshadowing_worker_token: &str,
    object_profile_worker_addr: SocketAddr,
    object_profile_worker_token: &str,
    scene_profile_worker_addr: SocketAddr,
    scene_profile_worker_token: &str,
    profile_material_worker_addr: SocketAddr,
    profile_material_worker_token: &str,
    project_status_worker_addr: SocketAddr,
    project_status_worker_token: &str,
    portrait_worker_addr: SocketAddr,
    portrait_worker_token: &str,
    source_snapshot_worker_addr: SocketAddr,
    source_snapshot_worker_token: &str,
    theme_read_worker_addr: SocketAddr,
    theme_read_worker_token: &str,
    project_read_worker_addr: SocketAddr,
    project_read_worker_token: &str,
    import_review_read_worker_addr: SocketAddr,
    import_review_read_worker_token: &str,
    inspiration_history_worker_addr: SocketAddr,
    inspiration_history_worker_token: &str,
    project_access_worker_addr: SocketAddr,
    project_access_worker_token: &str,
    project_collaboration_worker_addr: SocketAddr,
    project_collaboration_worker_token: &str,
    project_mutation_worker_addr: SocketAddr,
    project_mutation_worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    command
        .env("A4N_CONTROL_ADDR", address.to_string())
        .env(AGENT_WORKER_TOKEN_ENV, worker_token)
        .env(
            NOVEL_PLANNING_WORKER_URL_ENV,
            format!("http://{novel_planning_worker_addr}"),
        )
        .env(NOVEL_PLANNING_WORKER_TOKEN_ENV, novel_planning_worker_token)
        .env(
            ARTIFACT_MEDIA_WORKER_URL_ENV,
            format!("http://{artifact_media_worker_addr}"),
        )
        .env(ARTIFACT_MEDIA_WORKER_TOKEN_ENV, artifact_media_worker_token)
        .env(
            OUTLINE_WORKER_URL_ENV,
            format!("http://{outline_worker_addr}"),
        )
        .env(OUTLINE_WORKER_TOKEN_ENV, outline_worker_token)
        .env(
            BLUEPRINT_WORKER_URL_ENV,
            format!("http://{blueprint_worker_addr}"),
        )
        .env(BLUEPRINT_WORKER_TOKEN_ENV, blueprint_worker_token)
        .env(
            ASSET_GRAPH_WORKER_URL_ENV,
            format!("http://{asset_graph_worker_addr}"),
        )
        .env(ASSET_GRAPH_WORKER_TOKEN_ENV, asset_graph_worker_token)
        .env(
            FORESHADOWING_WORKER_URL_ENV,
            format!("http://{foreshadowing_worker_addr}"),
        )
        .env(FORESHADOWING_WORKER_TOKEN_ENV, foreshadowing_worker_token)
        .env(
            OBJECT_PROFILE_WORKER_URL_ENV,
            format!("http://{object_profile_worker_addr}"),
        )
        .env(OBJECT_PROFILE_WORKER_TOKEN_ENV, object_profile_worker_token)
        .env(
            SCENE_PROFILE_WORKER_URL_ENV,
            format!("http://{scene_profile_worker_addr}"),
        )
        .env(SCENE_PROFILE_WORKER_TOKEN_ENV, scene_profile_worker_token)
        .env(
            PROFILE_MATERIAL_WORKER_URL_ENV,
            format!("http://{profile_material_worker_addr}"),
        )
        .env(
            PROFILE_MATERIAL_WORKER_TOKEN_ENV,
            profile_material_worker_token,
        )
        .env(
            PROJECT_STATUS_WORKER_URL_ENV,
            format!("http://{project_status_worker_addr}"),
        )
        .env(PROJECT_STATUS_WORKER_TOKEN_ENV, project_status_worker_token)
        .env(
            PORTRAIT_WORKER_URL_ENV,
            format!("http://{portrait_worker_addr}"),
        )
        .env(PORTRAIT_WORKER_TOKEN_ENV, portrait_worker_token)
        .env(
            SOURCE_SNAPSHOT_WORKER_URL_ENV,
            format!("http://{source_snapshot_worker_addr}"),
        )
        .env(
            SOURCE_SNAPSHOT_WORKER_TOKEN_ENV,
            source_snapshot_worker_token,
        )
        .env(
            THEME_READ_WORKER_URL_ENV,
            format!("http://{theme_read_worker_addr}"),
        )
        .env(THEME_READ_WORKER_TOKEN_ENV, theme_read_worker_token)
        .env(
            PROJECT_READ_WORKER_URL_ENV,
            format!("http://{project_read_worker_addr}"),
        )
        .env(PROJECT_READ_WORKER_TOKEN_ENV, project_read_worker_token)
        .env(
            IMPORT_REVIEW_READ_WORKER_URL_ENV,
            format!("http://{import_review_read_worker_addr}"),
        )
        .env(
            IMPORT_REVIEW_READ_WORKER_TOKEN_ENV,
            import_review_read_worker_token,
        )
        .env(
            INSPIRATION_HISTORY_WORKER_URL_ENV,
            format!("http://{inspiration_history_worker_addr}"),
        )
        .env(
            INSPIRATION_HISTORY_WORKER_TOKEN_ENV,
            inspiration_history_worker_token,
        )
        .env(
            PROJECT_ACCESS_WORKER_URL_ENV,
            format!("http://{project_access_worker_addr}"),
        )
        .env(PROJECT_ACCESS_WORKER_TOKEN_ENV, project_access_worker_token)
        .env(
            PROJECT_COLLABORATION_WORKER_URL_ENV,
            format!("http://{project_collaboration_worker_addr}"),
        )
        .env(
            PROJECT_COLLABORATION_WORKER_TOKEN_ENV,
            project_collaboration_worker_token,
        )
        .env(
            PROJECT_MUTATION_WORKER_URL_ENV,
            format!("http://{project_mutation_worker_addr}"),
        )
        .env(
            PROJECT_MUTATION_WORKER_TOKEN_ENV,
            project_mutation_worker_token,
        )
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(CONTROL_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Control 失败: {}: {error}", binary.display()))
}

fn spawn_novel_planning_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = novel_planning_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(NOVEL_PLANNING_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Novel Planning worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_artifact_media_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = artifact_media_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(ARTIFACT_MEDIA_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(ARTIFACT_MEDIA_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Artifact/Media worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_outline_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = outline_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(OUTLINE_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(OUTLINE_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Outline worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_blueprint_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = blueprint_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(BLUEPRINT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(BLUEPRINT_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Blueprint worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_asset_graph_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = asset_graph_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(ASSET_GRAPH_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(ASSET_GRAPH_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Asset Graph worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_foreshadowing_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = foreshadowing_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(FORESHADOWING_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(FORESHADOWING_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Foreshadowing worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_object_profile_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = object_profile_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(OBJECT_PROFILE_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(OBJECT_PROFILE_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Object Profile worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_scene_profile_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = scene_profile_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(SCENE_PROFILE_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(SCENE_PROFILE_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Scene Profile worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_profile_material_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = profile_material_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROFILE_MATERIAL_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROFILE_MATERIAL_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Profile Material worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_project_status_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_status_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_STATUS_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_STATUS_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Status worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_theme_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = theme_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(THEME_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(THEME_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Theme Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_project_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_import_review_read_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = import_review_read_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(IMPORT_REVIEW_READ_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(IMPORT_REVIEW_READ_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Import Review Read worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_inspiration_history_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = inspiration_history_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(INSPIRATION_HISTORY_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(INSPIRATION_HISTORY_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Inspiration History worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_project_access_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_access_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_ACCESS_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_ACCESS_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Access worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_project_collaboration_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_collaboration_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_COLLABORATION_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_COLLABORATION_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Collaboration worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_project_mutation_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = project_mutation_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PROJECT_MUTATION_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PROJECT_MUTATION_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Project Mutation worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_portrait_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = portrait_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(PORTRAIT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(PORTRAIT_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Portrait worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_source_snapshot_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let metadata = source_snapshot_worker_metadata()?;
    command
        .env(metadata.addr_env.as_str(), address.to_string())
        .env(SOURCE_SNAPSHOT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(SOURCE_SNAPSHOT_WORKER_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 A4N Source Snapshot worker 失败: {}: {error}",
            binary.display()
        )
    })
}

fn spawn_edge(binary: &Path, control_addr: SocketAddr) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    command
        .env("A4N_CONTROL_ADDR", control_addr.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(EDGE_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Edge 失败: {}: {error}", binary.display()))
}

fn spawn_agent_worker(
    binary: &Path,
    address: SocketAddr,
    worker_address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    let worker_addr_env = agent_worker_metadata()?.addr_env.clone();
    command
        .env("A4N_CONTROL_ADDR", address.to_string())
        .env(worker_addr_env, worker_address.to_string())
        .env(AGENT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env("A4N_AGENT_WORKER_PROCESS", "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!("启动 A4N Agent worker 失败: {}: {error}", binary.display())
    })
}

fn wait_for_agent_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = agent_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        AGENT_WORKER_AUTH_HEADER,
        None,
        &required,
        "A4N Agent worker",
    )
}

fn wait_for_novel_planning_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = novel_planning_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(NOVEL_PLANNING_WORKER_CATALOG_NAME),
        &required,
        "A4N Novel Planning worker",
    )
}

fn wait_for_artifact_media_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = artifact_media_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(ARTIFACT_MEDIA_WORKER_CATALOG_NAME),
        &required,
        "A4N Artifact/Media worker",
    )
}

fn wait_for_outline_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = outline_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(OUTLINE_WORKER_CATALOG_NAME),
        &required,
        "A4N Outline worker",
    )
}

fn wait_for_blueprint_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = blueprint_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(BLUEPRINT_WORKER_CATALOG_NAME),
        &required,
        "A4N Blueprint worker",
    )
}

fn wait_for_asset_graph_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = asset_graph_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(ASSET_GRAPH_WORKER_CATALOG_NAME),
        &required,
        "A4N Asset Graph worker",
    )
}

fn wait_for_foreshadowing_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = foreshadowing_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(FORESHADOWING_WORKER_CATALOG_NAME),
        &required,
        "A4N Foreshadowing worker",
    )
}

fn wait_for_object_profile_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = object_profile_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(OBJECT_PROFILE_WORKER_CATALOG_NAME),
        &required,
        "A4N Object Profile worker",
    )
}

fn wait_for_scene_profile_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = scene_profile_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(SCENE_PROFILE_WORKER_CATALOG_NAME),
        &required,
        "A4N Scene Profile worker",
    )
}

fn wait_for_profile_material_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = profile_material_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROFILE_MATERIAL_WORKER_CATALOG_NAME),
        &required,
        "A4N Profile Material worker",
    )
}

fn wait_for_project_status_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_status_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_STATUS_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Status worker",
    )
}

fn wait_for_theme_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = theme_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(THEME_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Theme Read worker",
    )
}

fn wait_for_project_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Read worker",
    )
}

fn wait_for_import_review_read_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = import_review_read_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(IMPORT_REVIEW_READ_WORKER_CATALOG_NAME),
        &required,
        "A4N Import Review Read worker",
    )
}

fn wait_for_inspiration_history_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = inspiration_history_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(INSPIRATION_HISTORY_WORKER_CATALOG_NAME),
        &required,
        "A4N Inspiration History worker",
    )
}

fn wait_for_project_access_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_access_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_ACCESS_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Access worker",
    )
}

fn wait_for_project_collaboration_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_collaboration_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_COLLABORATION_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Collaboration worker",
    )
}

fn wait_for_project_mutation_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = project_mutation_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PROJECT_MUTATION_WORKER_CATALOG_NAME),
        &required,
        "A4N Project Mutation worker",
    )
}

fn wait_for_portrait_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = portrait_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(PORTRAIT_WORKER_CATALOG_NAME),
        &required,
        "A4N Portrait worker",
    )
}

fn wait_for_source_snapshot_worker(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<()> {
    let metadata = source_snapshot_worker_metadata()?;
    let required = metadata
        .capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    wait_for_worker_contract(
        child,
        address,
        worker_token,
        WORKER_AUTH_HEADER,
        Some(SOURCE_SNAPSHOT_WORKER_CATALOG_NAME),
        &required,
        "A4N Source Snapshot worker",
    )
}

fn wait_for_worker_contract(
    child: &mut Child,
    address: SocketAddr,
    worker_token: &str,
    auth_header: &str,
    expected_worker_id: Option<&str>,
    required_capabilities: &[&str],
    worker_name: &str,
) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(WORKER_READY_CONNECT_TIMEOUT)
        .timeout(WORKER_READY_CONNECT_TIMEOUT)
        .build()
        .map_err(|error| anyhow::anyhow!("初始化 worker health client 失败: {error}"))?;
    let started = Instant::now();
    let health_url = format!("http://{address}{WORKER_HEALTH_PATH}");
    let capabilities_url = format!("http://{address}{WORKER_CAPABILITIES_PATH}");
    let mut last_error = None;
    while started.elapsed() < WORKER_READY_TIMEOUT {
        if let Some(status) = child.try_wait()? {
            anyhow::bail!(
                "{worker_name} 在健康检查通过前退出: {status}; 最后一次错误: {}",
                last_error
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "无".to_string())
            );
        }

        match client
            .get(&health_url)
            .header(auth_header, worker_token)
            .send()
        {
            Ok(response) if response.status().is_success() => {
                let health = response.json::<WorkerHealth>().map_err(|error| {
                    anyhow::anyhow!("{worker_name} health 响应不是有效合同: {error}")
                })?;
                if let Some(expected_worker_id) = expected_worker_id {
                    health.validate_for(expected_worker_id).map_err(|error| {
                        anyhow::anyhow!("{worker_name} health 合同无效: {error}")
                    })?;
                } else {
                    health.validate().map_err(|error| {
                        anyhow::anyhow!("{worker_name} health 合同无效: {error}")
                    })?;
                }
                if health.status == WorkerHealthStatus::Healthy {
                    let capabilities = client
                        .get(&capabilities_url)
                        .header(auth_header, worker_token)
                        .send()
                        .map_err(|error| {
                            anyhow::anyhow!("{worker_name} capabilities 请求失败: {error}")
                        })?
                        .json::<WorkerCapabilities>()
                        .map_err(|error| {
                            anyhow::anyhow!("{worker_name} capabilities 响应不是有效合同: {error}")
                        })?;
                    if let Some(expected_worker_id) = expected_worker_id {
                        capabilities
                            .validate_for(expected_worker_id)
                            .map_err(|error| {
                                anyhow::anyhow!("{worker_name} capabilities 合同无效: {error}")
                            })?;
                    } else {
                        capabilities.validate().map_err(|error| {
                            anyhow::anyhow!("{worker_name} capabilities 合同无效: {error}")
                        })?;
                    }
                    for required in required_capabilities {
                        if !capabilities
                            .capabilities
                            .iter()
                            .any(|capability| capability.enabled && capability.name == *required)
                        {
                            anyhow::bail!("{worker_name} capabilities 缺少已启用操作: {required}");
                        }
                    }
                    return Ok(());
                }
                last_error = Some(anyhow::anyhow!(
                    "{worker_name} health 状态为 {:?}",
                    health.status
                ));
            }
            Ok(response) => {
                last_error = Some(anyhow::anyhow!(
                    "{worker_name} health 返回 {}",
                    response.status()
                ));
            }
            Err(error) => last_error = Some(anyhow::anyhow!(error)),
        }
        std::thread::sleep(WORKER_READY_RETRY_INTERVAL);
    }

    anyhow::bail!(
        "{worker_name} 在 {} 秒内未通过健康检查 {health_url}: {}",
        WORKER_READY_TIMEOUT.as_secs(),
        last_error
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "无".to_string())
    )
}

fn control_binary() -> anyhow::Result<PathBuf> {
    child_binary(CONTROL_BINARY_ENV, "a4n-control")
}

fn edge_binary() -> anyhow::Result<PathBuf> {
    child_binary(EDGE_BINARY_ENV, "a4n-edge")
}

fn agent_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(AGENT_WORKER_BINARY_ENV, &agent_worker_metadata()?.binary)
}

fn novel_planning_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        NOVEL_PLANNING_WORKER_BINARY_ENV,
        &novel_planning_worker_metadata()?.binary,
    )
}

fn artifact_media_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        ARTIFACT_MEDIA_WORKER_BINARY_ENV,
        &artifact_media_worker_metadata()?.binary,
    )
}

fn outline_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        OUTLINE_WORKER_BINARY_ENV,
        &outline_worker_metadata()?.binary,
    )
}

fn blueprint_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        BLUEPRINT_WORKER_BINARY_ENV,
        &blueprint_worker_metadata()?.binary,
    )
}

fn asset_graph_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        ASSET_GRAPH_WORKER_BINARY_ENV,
        &asset_graph_worker_metadata()?.binary,
    )
}

fn foreshadowing_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        FORESHADOWING_WORKER_BINARY_ENV,
        &foreshadowing_worker_metadata()?.binary,
    )
}

fn object_profile_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        OBJECT_PROFILE_WORKER_BINARY_ENV,
        &object_profile_worker_metadata()?.binary,
    )
}

fn scene_profile_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        SCENE_PROFILE_WORKER_BINARY_ENV,
        &scene_profile_worker_metadata()?.binary,
    )
}

fn profile_material_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROFILE_MATERIAL_WORKER_BINARY_ENV,
        &profile_material_worker_metadata()?.binary,
    )
}

fn project_status_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_STATUS_WORKER_BINARY_ENV,
        &project_status_worker_metadata()?.binary,
    )
}

fn portrait_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PORTRAIT_WORKER_BINARY_ENV,
        &portrait_worker_metadata()?.binary,
    )
}

fn source_snapshot_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        SOURCE_SNAPSHOT_WORKER_BINARY_ENV,
        &source_snapshot_worker_metadata()?.binary,
    )
}

fn theme_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        THEME_READ_WORKER_BINARY_ENV,
        &theme_read_worker_metadata()?.binary,
    )
}

fn project_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_READ_WORKER_BINARY_ENV,
        &project_read_worker_metadata()?.binary,
    )
}

fn import_review_read_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        IMPORT_REVIEW_READ_WORKER_BINARY_ENV,
        &import_review_read_worker_metadata()?.binary,
    )
}

fn inspiration_history_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        INSPIRATION_HISTORY_WORKER_BINARY_ENV,
        &inspiration_history_worker_metadata()?.binary,
    )
}

fn project_access_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_ACCESS_WORKER_BINARY_ENV,
        &project_access_worker_metadata()?.binary,
    )
}

fn project_collaboration_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_COLLABORATION_WORKER_BINARY_ENV,
        &project_collaboration_worker_metadata()?.binary,
    )
}

fn project_mutation_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(
        PROJECT_MUTATION_WORKER_BINARY_ENV,
        &project_mutation_worker_metadata()?.binary,
    )
}

fn child_binary(env_name: &str, name: &str) -> anyhow::Result<PathBuf> {
    if let Some(path) = env::var_os(env_name).map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        anyhow::bail!("{env_name} 指向的文件不存在: {}", path.display());
    }

    if let Some(path) = sibling_binary(name) {
        return Ok(path);
    }

    anyhow::bail!("未找到 {name}；请设置 {env_name} 或将其放在 a4n-server 同目录")
}

fn sibling_binary(name: &str) -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    let parent = executable.parent()?;
    let path = parent.join(executable_name(name));
    path.is_file().then_some(path)
}

#[cfg(windows)]
fn executable_name(name: &str) -> PathBuf {
    format!("{name}.exe").into()
}

#[cfg(not(windows))]
fn executable_name(name: &str) -> PathBuf {
    name.into()
}

fn worker_token() -> anyhow::Result<String> {
    worker_token_for_env(AGENT_WORKER_TOKEN_ENV)
}

fn worker_token_for_env(env_name: &str) -> anyhow::Result<String> {
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

fn stop_agent_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_novel_planning_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_artifact_media_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_outline_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_blueprint_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_asset_graph_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_foreshadowing_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_object_profile_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_scene_profile_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_profile_material_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_project_status_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_theme_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_project_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_import_review_read_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_inspiration_history_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_project_access_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_project_collaboration_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_project_mutation_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_portrait_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_source_snapshot_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_control(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_edge(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_child(child: &mut Child) -> anyhow::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
        for _ in 0..50 {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    child.kill()?;
    child.wait()?;
    Ok(())
}

fn finish_shutdown(
    edge_result: anyhow::Result<()>,
    worker_result: anyhow::Result<()>,
    control_result: anyhow::Result<()>,
    novel_planning_worker_result: anyhow::Result<()>,
    artifact_media_worker_result: anyhow::Result<()>,
    outline_worker_result: anyhow::Result<()>,
    blueprint_worker_result: anyhow::Result<()>,
    asset_graph_worker_result: anyhow::Result<()>,
    foreshadowing_worker_result: anyhow::Result<()>,
    object_profile_worker_result: anyhow::Result<()>,
    scene_profile_worker_result: anyhow::Result<()>,
    profile_material_worker_result: anyhow::Result<()>,
    project_status_worker_result: anyhow::Result<()>,
    portrait_worker_result: anyhow::Result<()>,
    source_snapshot_worker_result: anyhow::Result<()>,
    theme_read_worker_result: anyhow::Result<()>,
    project_read_worker_result: anyhow::Result<()>,
    import_review_read_worker_result: anyhow::Result<()>,
    inspiration_history_worker_result: anyhow::Result<()>,
    project_access_worker_result: anyhow::Result<()>,
    project_collaboration_worker_result: anyhow::Result<()>,
    project_mutation_worker_result: anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut failures = Vec::new();
    for (name, result) in [
        ("Edge", edge_result),
        ("Agent worker", worker_result),
        ("Control", control_result),
        ("Novel Planning worker", novel_planning_worker_result),
        ("Artifact/Media worker", artifact_media_worker_result),
        ("Outline worker", outline_worker_result),
        ("Blueprint worker", blueprint_worker_result),
        ("Asset Graph worker", asset_graph_worker_result),
        ("Foreshadowing worker", foreshadowing_worker_result),
        ("Object Profile worker", object_profile_worker_result),
        ("Scene Profile worker", scene_profile_worker_result),
        ("Profile Material worker", profile_material_worker_result),
        ("Project Status worker", project_status_worker_result),
        ("Portrait worker", portrait_worker_result),
        ("Source Snapshot worker", source_snapshot_worker_result),
        ("Theme Read worker", theme_read_worker_result),
        ("Project Read worker", project_read_worker_result),
        (
            "Import Review Read worker",
            import_review_read_worker_result,
        ),
        (
            "Inspiration History worker",
            inspiration_history_worker_result,
        ),
        ("Project Access worker", project_access_worker_result),
        (
            "Project Collaboration worker",
            project_collaboration_worker_result,
        ),
        ("Project Mutation worker", project_mutation_worker_result),
    ] {
        if let Err(error) = result {
            failures.push(format!("{name}: {error:#}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(failures.join("；"));
    }
}

#[cfg(target_os = "linux")]
fn configure_parent_death_signal(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn prctl(option: i32, ...) -> i32;
            }
            const PR_SET_PDEATHSIG: i32 = 1;
            const SIGTERM: i32 = 15;
            if prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_parent_death_signal(_: &mut Command) {}

#[cfg(all(test, unix))]
mod tests {
    use super::wait_for_control;
    use std::{
        net::TcpListener,
        process::{Child, Command},
    };

    fn live_child() -> Child {
        Command::new("sh")
            .args(["-c", "sleep 5"])
            .spawn()
            .expect("test child should start")
    }

    fn exited_child() -> Child {
        Command::new("sh")
            .args(["-c", "exit 7"])
            .spawn()
            .expect("test child should start")
    }

    fn novel_planning_fixture(
        tempdir: &tempfile::TempDir,
    ) -> (Child, std::path::PathBuf, super::BinaryFingerprint) {
        let child = live_child();
        let binary = tempdir.path().join("a4n-novel-planning-worker");
        std::fs::write(&binary, b"novel-planning-worker")
            .expect("novel planning worker fixture should be written");
        let fingerprint =
            super::binary_fingerprint(&binary).expect("novel planning fingerprint should work");
        (child, binary, fingerprint)
    }

    fn artifact_media_fixture(
        tempdir: &tempfile::TempDir,
    ) -> (Child, std::path::PathBuf, super::BinaryFingerprint) {
        let child = live_child();
        let binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&binary, b"artifact-media-worker")
            .expect("artifact media worker fixture should be written");
        let fingerprint =
            super::binary_fingerprint(&binary).expect("artifact media fingerprint should work");
        (child, binary, fingerprint)
    }

    #[test]
    fn waits_for_control_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
        let address = listener
            .local_addr()
            .expect("test listener should expose address");
        let mut child = live_child();

        wait_for_control(&mut child, address).expect("listener should be considered ready");

        child.kill().expect("test child should stop");
        child.wait().expect("test child should be reaped");
    }

    #[test]
    fn reports_control_exit_before_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
        let address = listener
            .local_addr()
            .expect("test listener should expose address");
        drop(listener);
        let mut child = exited_child();

        let error = wait_for_control(&mut child, address).expect_err("exited Control must fail");
        assert!(error.to_string().contains("listener 就绪前退出"));
    }

    #[test]
    fn reports_child_exit_while_edge_is_running() {
        let mut edge = live_child();
        let mut control = exited_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let (
            mut novel_planning_worker,
            novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
        ) = novel_planning_fixture(&tempdir);
        let (
            mut artifact_media_worker,
            artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
        ) = artifact_media_fixture(&tempdir);

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ControlExited(_))
        ));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        novel_planning_worker
            .kill()
            .expect("test novel planning worker should stop");
        novel_planning_worker
            .wait()
            .expect("test novel planning worker should be reaped");
        artifact_media_worker
            .kill()
            .expect("test artifact media worker should stop");
        artifact_media_worker
            .wait()
            .expect("test artifact media worker should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }

    #[test]
    fn detects_replaced_worker_binary() {
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&worker_binary, b"old-worker")
            .expect("old worker fixture should be written");
        let old = super::binary_fingerprint(&worker_binary).expect("old fingerprint should work");
        std::fs::write(&worker_binary, b"new-worker-with-a-different-size")
            .expect("new worker fixture should be written");
        let new = super::binary_fingerprint(&worker_binary).expect("new fingerprint should work");

        assert_ne!(old, new);
    }

    #[test]
    fn control_binary_change_only_selects_control_replacement() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&control_binary, b"old-control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let (
            mut novel_planning_worker,
            novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
        ) = novel_planning_fixture(&tempdir);
        let (
            mut artifact_media_worker,
            artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
        ) = artifact_media_fixture(&tempdir);
        std::fs::write(&control_binary, b"new-control").expect("control replacement should work");

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ControlChanged)
        ));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        novel_planning_worker
            .kill()
            .expect("test novel planning worker should stop");
        novel_planning_worker
            .wait()
            .expect("test novel planning worker should be reaped");
        artifact_media_worker
            .kill()
            .expect("test artifact media worker should stop");
        artifact_media_worker
            .wait()
            .expect("test artifact media worker should be reaped");
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }

    #[test]
    fn worker_binary_change_only_selects_worker_replacement() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"old-worker").expect("worker fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let (
            mut novel_planning_worker,
            novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
        ) = novel_planning_fixture(&tempdir);
        let (
            mut artifact_media_worker,
            artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
        ) = artifact_media_fixture(&tempdir);
        std::fs::write(&worker_binary, b"new-worker").expect("worker replacement should work");

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::WorkerChanged)
        ));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        novel_planning_worker
            .kill()
            .expect("test novel planning worker should stop");
        novel_planning_worker
            .wait()
            .expect("test novel planning worker should be reaped");
        artifact_media_worker
            .kill()
            .expect("test artifact media worker should stop");
        artifact_media_worker
            .wait()
            .expect("test artifact media worker should be reaped");
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }

    #[test]
    fn edge_binary_change_only_selects_edge_replacement() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        std::fs::write(&edge_binary, b"old-edge").expect("edge fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let (
            mut novel_planning_worker,
            novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
        ) = novel_planning_fixture(&tempdir);
        let (
            mut artifact_media_worker,
            artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
        ) = artifact_media_fixture(&tempdir);
        std::fs::write(&edge_binary, b"new-edge").expect("edge replacement should work");

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(result, Some(super::ChildWatchEvent::EdgeChanged)));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        novel_planning_worker
            .kill()
            .expect("test novel planning worker should stop");
        novel_planning_worker
            .wait()
            .expect("test novel planning worker should be reaped");
        artifact_media_worker
            .kill()
            .expect("test artifact media worker should stop");
        artifact_media_worker
            .wait()
            .expect("test artifact media worker should be reaped");
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }

    #[test]
    fn novel_planning_worker_binary_change_only_selects_novel_planning_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_media_worker_binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        std::fs::write(&novel_binary, b"old-novel").expect("novel fixture should be written");
        std::fs::write(&artifact_media_worker_binary, b"artifact-media-worker")
            .expect("artifact media fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let novel_fingerprint =
            super::binary_fingerprint(&novel_binary).expect("novel fingerprint should work");
        let artifact_media_worker_fingerprint =
            super::binary_fingerprint(&artifact_media_worker_binary)
                .expect("artifact media fingerprint should work");
        std::fs::write(&novel_binary, b"new-novel").expect("novel replacement should be written");

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::NovelPlanningWorkerChanged)
        ));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
        novel_planning_worker
            .kill()
            .expect("test novel planning worker should stop");
        novel_planning_worker
            .wait()
            .expect("test novel planning worker should be reaped");
        artifact_media_worker
            .kill()
            .expect("test artifact media worker should stop");
        artifact_media_worker
            .wait()
            .expect("test artifact media worker should be reaped");
    }

    #[test]
    fn artifact_media_worker_binary_change_only_selects_artifact_media_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut artifact_media_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        let (mut novel_planning_worker, novel_binary, novel_fingerprint) =
            novel_planning_fixture(&tempdir);
        let artifact_media_worker_binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&artifact_media_worker_binary, b"old-artifact")
            .expect("artifact fixture should be written");
        let artifact_media_worker_fingerprint =
            super::binary_fingerprint(&artifact_media_worker_binary)
                .expect("artifact fingerprint should work");
        std::fs::write(&artifact_media_worker_binary, b"new-artifact")
            .expect("artifact replacement should be written");

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ArtifactMediaWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn foreshadowing_worker_binary_change_only_selects_foreshadowing_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut foreshadowing_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let foreshadowing_binary = tempdir.path().join("a4n-foreshadowing-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&foreshadowing_binary, b"old-foreshadowing".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let foreshadowing_fingerprint = super::binary_fingerprint(&foreshadowing_binary).unwrap();
        std::fs::write(&foreshadowing_binary, b"new-foreshadowing")
            .expect("foreshadowing replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            Some((
                &mut foreshadowing_worker,
                &foreshadowing_binary,
                foreshadowing_fingerprint,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ForeshadowingWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut foreshadowing_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn profile_worker_binary_changes_select_only_changed_profile_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut object_profile_worker = live_child();
        let mut scene_profile_worker = live_child();
        let mut profile_material_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let object_binary = tempdir.path().join("a4n-object-profile-worker");
        let scene_binary = tempdir.path().join("a4n-scene-profile-worker");
        let profile_material_binary = tempdir.path().join("a4n-profile-material-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&object_binary, b"old-object".as_slice()),
            (&scene_binary, b"old-scene".as_slice()),
            (&profile_material_binary, b"old-profile-material".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let object_fingerprint = super::binary_fingerprint(&object_binary).unwrap();
        let scene_fingerprint = super::binary_fingerprint(&scene_binary).unwrap();
        let profile_material_fingerprint =
            super::binary_fingerprint(&profile_material_binary).unwrap();
        std::fs::write(&object_binary, b"new-object")
            .expect("object profile replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            Some((
                &mut object_profile_worker,
                &object_binary,
                object_fingerprint,
            )),
            Some((&mut scene_profile_worker, &scene_binary, scene_fingerprint)),
            Some((
                &mut profile_material_worker,
                &profile_material_binary,
                profile_material_fingerprint,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");
        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ObjectProfileWorkerChanged)
        ));

        let object_fingerprint = super::binary_fingerprint(&object_binary).unwrap();
        std::fs::write(&profile_material_binary, b"new-profile-material")
            .expect("profile material replacement should be written");
        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            Some((
                &mut object_profile_worker,
                &object_binary,
                object_fingerprint,
            )),
            Some((&mut scene_profile_worker, &scene_binary, scene_fingerprint)),
            Some((
                &mut profile_material_worker,
                &profile_material_binary,
                profile_material_fingerprint,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");
        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ProfileMaterialWorkerChanged)
        ));

        let profile_material_fingerprint =
            super::binary_fingerprint(&profile_material_binary).unwrap();
        std::fs::write(&scene_binary, b"new-scene")
            .expect("scene profile replacement should be written");
        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            Some((
                &mut object_profile_worker,
                &object_binary,
                object_fingerprint,
            )),
            Some((&mut scene_profile_worker, &scene_binary, scene_fingerprint)),
            Some((
                &mut profile_material_worker,
                &profile_material_binary,
                profile_material_fingerprint,
            )),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");
        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::SceneProfileWorkerChanged)
        ));

        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut object_profile_worker,
            &mut scene_profile_worker,
            &mut profile_material_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn outline_and_blueprint_worker_changes_select_only_changed_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut outline_worker = live_child();
        let mut blueprint_worker = live_child();
        let mut asset_graph_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let outline_binary = tempdir.path().join("a4n-outline-worker");
        let blueprint_binary = tempdir.path().join("a4n-blueprint-worker");
        let asset_graph_binary = tempdir.path().join("a4n-asset-graph-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&outline_binary, b"old-outline".as_slice()),
            (&blueprint_binary, b"old-blueprint".as_slice()),
            (&asset_graph_binary, b"old-asset-graph".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let outline_fingerprint = super::binary_fingerprint(&outline_binary).unwrap();
        let blueprint_fingerprint = super::binary_fingerprint(&blueprint_binary).unwrap();
        let asset_graph_fingerprint = super::binary_fingerprint(&asset_graph_binary).unwrap();
        std::fs::write(&outline_binary, b"new-outline").expect("outline replacement should write");

        let result = super::wait_for_edge_or_child_with_outline(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            Some((&mut outline_worker, &outline_binary, outline_fingerprint)),
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::OutlineWorkerChanged)
        ));

        std::fs::write(&blueprint_binary, b"new-blueprint")
            .expect("blueprint replacement should write");
        let outline_fingerprint = super::binary_fingerprint(&outline_binary).unwrap();
        let result = super::wait_for_edge_or_child_with_outline(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            Some((&mut outline_worker, &outline_binary, outline_fingerprint)),
            Some((
                &mut blueprint_worker,
                &blueprint_binary,
                blueprint_fingerprint,
            )),
            Some((
                &mut asset_graph_worker,
                &asset_graph_binary,
                asset_graph_fingerprint,
            )),
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::BlueprintWorkerChanged)
        ));
        std::fs::write(&asset_graph_binary, b"new-asset-graph")
            .expect("asset graph replacement should write");
        let blueprint_fingerprint = super::binary_fingerprint(&blueprint_binary).unwrap();
        let result = super::wait_for_edge_or_child_with_outline(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            Some((&mut outline_worker, &outline_binary, outline_fingerprint)),
            Some((
                &mut blueprint_worker,
                &blueprint_binary,
                blueprint_fingerprint,
            )),
            Some((
                &mut asset_graph_worker,
                &asset_graph_binary,
                asset_graph_fingerprint,
            )),
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");
        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::AssetGraphWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut outline_worker,
            &mut blueprint_worker,
            &mut asset_graph_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn reload_marker_is_optional_and_detects_publication() {
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let marker = tempdir.path().join("runtime-workers.ready");

        assert_eq!(
            super::optional_binary_fingerprint(Some(&marker)).expect("missing marker is valid"),
            None
        );
        std::fs::write(&marker, b"1\n").expect("marker should be published");
        let first = super::optional_binary_fingerprint(Some(&marker))
            .expect("published marker should be fingerprinted")
            .expect("published marker should exist");
        std::fs::write(&marker, b"2\n").expect("marker should be republished");
        let second = super::optional_binary_fingerprint(Some(&marker))
            .expect("republished marker should be fingerprinted")
            .expect("republished marker should exist");

        assert_ne!(first, second);
    }

    #[test]
    fn marker_change_without_binary_change_does_not_select_a_process() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut artifact_media_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let marker = tempdir.path().join("runtime-workers.ready");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");
        let edge_binary = tempdir.path().join("a4n-edge");
        let artifact_media_worker_binary = tempdir.path().join("a4n-artifact-media-worker");
        std::fs::write(&edge_binary, b"edge").expect("edge fixture should be written");
        let edge_fingerprint =
            super::binary_fingerprint(&edge_binary).expect("edge fingerprint should work");
        std::fs::write(&artifact_media_worker_binary, b"artifact-media-worker")
            .expect("artifact media fixture should be written");
        let artifact_media_worker_fingerprint =
            super::binary_fingerprint(&artifact_media_worker_binary)
                .expect("artifact media fingerprint should work");
        let (
            mut novel_planning_worker,
            novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
        ) = novel_planning_fixture(&tempdir);
        let marker_fingerprint = None;
        std::fs::write(&marker, b"published").expect("marker should be published");

        let result = super::wait_for_edge_or_child(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_planning_worker_binary,
            novel_planning_worker_fingerprint,
            &mut artifact_media_worker,
            &artifact_media_worker_binary,
            artifact_media_worker_fingerprint,
            &edge_binary,
            edge_fingerprint,
            Some(&marker),
            marker_fingerprint,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ReloadMarkerChanged)
        ));
        edge.kill().expect("test edge should stop");
        edge.wait().expect("test edge should be reaped");
        novel_planning_worker
            .kill()
            .expect("test novel planning worker should stop");
        novel_planning_worker
            .wait()
            .expect("test novel planning worker should be reaped");
        artifact_media_worker
            .kill()
            .expect("test artifact media worker should stop");
        artifact_media_worker
            .wait()
            .expect("test artifact media worker should be reaped");
        control.kill().expect("test control should stop");
        control.wait().expect("test control should be reaped");
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }

    #[test]
    fn theme_read_worker_binary_changes_select_only_theme_read_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut theme_read_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let theme_binary = tempdir.path().join("a4n-theme-read-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&theme_binary, b"old-theme".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let theme_fingerprint = super::binary_fingerprint(&theme_binary).unwrap();
        std::fs::write(&theme_binary, b"new-theme").expect("theme replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((&mut theme_read_worker, &theme_binary, theme_fingerprint)),
            None,
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ThemeReadWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut theme_read_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn project_read_worker_binary_changes_select_only_project_read_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut project_read_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let project_binary = tempdir.path().join("a4n-project-read-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&project_binary, b"old-project".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let project_fingerprint = super::binary_fingerprint(&project_binary).unwrap();
        std::fs::write(&project_binary, b"new-project")
            .expect("project read replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((
                &mut project_read_worker,
                &project_binary,
                project_fingerprint,
            )),
            None,
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ProjectReadWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut project_read_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn import_review_read_worker_binary_changes_select_only_import_review_read_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut import_review_read_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let import_review_binary = tempdir.path().join("a4n-import-review-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&import_review_binary, b"old-import-review".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let import_review_fingerprint = super::binary_fingerprint(&import_review_binary).unwrap();
        std::fs::write(&import_review_binary, b"new-import-review")
            .expect("import review replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((
                &mut import_review_read_worker,
                &import_review_binary,
                import_review_fingerprint,
            )),
            None,
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ImportReviewReadWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut import_review_read_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }

    #[test]
    fn inspiration_history_worker_binary_changes_select_only_inspiration_history_worker() {
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = live_child();
        let mut novel_planning_worker = live_child();
        let mut artifact_media_worker = live_child();
        let mut inspiration_history_worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        let edge_binary = tempdir.path().join("a4n-edge");
        let novel_binary = tempdir.path().join("a4n-novel-planning-worker");
        let artifact_binary = tempdir.path().join("a4n-artifact-media-worker");
        let inspiration_binary = tempdir.path().join("a4n-inspiration-worker");
        for (path, content) in [
            (&control_binary, b"control".as_slice()),
            (&worker_binary, b"worker".as_slice()),
            (&edge_binary, b"edge".as_slice()),
            (&novel_binary, b"novel".as_slice()),
            (&artifact_binary, b"artifact".as_slice()),
            (&inspiration_binary, b"old-inspiration".as_slice()),
        ] {
            std::fs::write(path, content).expect("worker fixture should be written");
        }
        let control_fingerprint = super::binary_fingerprint(&control_binary).unwrap();
        let worker_fingerprint = super::binary_fingerprint(&worker_binary).unwrap();
        let edge_fingerprint = super::binary_fingerprint(&edge_binary).unwrap();
        let novel_fingerprint = super::binary_fingerprint(&novel_binary).unwrap();
        let artifact_fingerprint = super::binary_fingerprint(&artifact_binary).unwrap();
        let inspiration_fingerprint = super::binary_fingerprint(&inspiration_binary).unwrap();
        std::fs::write(&inspiration_binary, b"new-inspiration")
            .expect("inspiration replacement should be written");

        let result = super::wait_for_edge_or_child_with_workers(
            &mut edge,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
            &mut novel_planning_worker,
            &novel_binary,
            novel_fingerprint,
            &mut artifact_media_worker,
            &artifact_binary,
            artifact_fingerprint,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some((
                &mut inspiration_history_worker,
                &inspiration_binary,
                inspiration_fingerprint,
            )),
            None,
            None,
            &edge_binary,
            edge_fingerprint,
            None,
            None,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::InspirationHistoryWorkerChanged)
        ));
        for child in [
            &mut edge,
            &mut control,
            &mut worker,
            &mut novel_planning_worker,
            &mut artifact_media_worker,
            &mut inspiration_history_worker,
        ] {
            child.kill().expect("test child should stop");
            child.wait().expect("test child should be reaped");
        }
    }
}
